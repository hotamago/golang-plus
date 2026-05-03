import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

function getGoplusBinary(): string {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get<string>('binaryPath', 'goplus');
}

function getOutDir(): string {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get<string>('outDir', '.goplusgen');
}

interface SourceMapFile {
    version: number;
    generated: string;
    sources: string[];
    mappings: Array<{
        source: string;
        source_start: number;
        source_end: number;
        generated_start: number;
        generated_end: number;
        kind: string;
        name: string;
    }>;
}

/**
 * Find all `.go.map` files in the workspace's outDir.
 */
function findSourceMaps(workspaceRoot: string): string[] {
    const outDir = path.resolve(workspaceRoot, getOutDir());
    const results: string[] = [];

    function walk(dir: string): void {
        try {
            const entries = fs.readdirSync(dir, { withFileTypes: true });
            for (const entry of entries) {
                const fullPath = path.join(dir, entry.name);
                if (entry.isDirectory() && !entry.name.startsWith('.')) {
                    walk(fullPath);
                } else if (entry.isFile() && entry.name.endsWith('.go.map')) {
                    results.push(fullPath);
                }
            }
        } catch {
            // Directory may not exist
        }
    }

    walk(outDir);
    return results;
}

/**
 * Find the source map that contains a reference to a given .gp source file.
 */
function findSourceMapForGpFile(workspaceRoot: string, gpFilePath: string): string | undefined {
    const maps = findSourceMaps(workspaceRoot);
    const normalizedGp = gpFilePath.replace(/\\/g, '/').toLowerCase();

    for (const mapPath of maps) {
        try {
            const content = fs.readFileSync(mapPath, 'utf-8');
            const map: SourceMapFile = JSON.parse(content);
            const hasSource = map.sources.some(s =>
                s.replace(/\\/g, '/').toLowerCase().includes(
                    normalizedGp.split('/').slice(-2).join('/')
                )
            );
            if (hasSource) {
                return mapPath;
            }
        } catch {
            // Skip invalid maps
        }
    }
    return undefined;
}

/**
 * Find the source map for a generated .go file (look for .go.map next to it).
 */
function findSourceMapForGoFile(goFilePath: string): string | undefined {
    const mapPath = goFilePath + '.map';
    if (fs.existsSync(mapPath)) {
        return mapPath;
    }
    return undefined;
}

function execNavigate(args: string[], cwd: string): Promise<string> {
    const binary = getGoplusBinary();
    return new Promise((resolve, reject) => {
        cp.execFile(binary, args, { cwd, timeout: 10000 }, (err, stdout, stderr) => {
            if (err) {
                reject(new Error(stderr || err.message));
                return;
            }
            resolve(stdout.trim());
        });
    });
}

function parseLocationOutput(output: string): { file: string; line: number; column: number } | undefined {
    // Output format: <file>:<line>:<column>
    const parts = output.split(':');
    if (parts.length < 3) {
        return undefined;
    }
    // Handle Windows paths with drive letter (e.g., D:\path:10:5)
    let file: string;
    let lineStr: string;
    let colStr: string;

    if (parts.length >= 4 && /^[a-zA-Z]$/.test(parts[0])) {
        // Windows path: D:\path\file.go:10:5
        file = parts[0] + ':' + parts[1];
        lineStr = parts[2];
        colStr = parts[3];
    } else {
        file = parts.slice(0, -2).join(':');
        lineStr = parts[parts.length - 2];
        colStr = parts[parts.length - 1];
    }

    const line = parseInt(lineStr, 10);
    const column = parseInt(colStr, 10);

    if (isNaN(line) || isNaN(column)) {
        return undefined;
    }

    return { file, line, column };
}

export async function navigateToGo(
    document: vscode.TextDocument,
    position: vscode.Position
): Promise<vscode.Location | undefined> {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (!workspaceFolder) {
        return undefined;
    }

    const gpFilePath = document.uri.fsPath;
    const mapPath = findSourceMapForGpFile(workspaceFolder.uri.fsPath, gpFilePath);
    if (!mapPath) {
        return undefined;
    }

    try {
        const output = await execNavigate(
            [
                'navigate',
                '--source-map', mapPath,
                '--file', gpFilePath,
                '--line', String(position.line + 1),
                '--column', String(position.character + 1),
            ],
            workspaceFolder.uri.fsPath
        );

        const loc = parseLocationOutput(output);
        if (!loc) {
            return undefined;
        }

        return new vscode.Location(
            vscode.Uri.file(loc.file),
            new vscode.Position(loc.line - 1, loc.column - 1)
        );
    } catch {
        return undefined;
    }
}

export async function navigateToGp(
    document: vscode.TextDocument,
    position: vscode.Position
): Promise<vscode.Location | undefined> {
    const goFilePath = document.uri.fsPath;

    if (!path.basename(goFilePath).startsWith('zz_goplus_gen')) {
        return undefined;
    }

    const mapPath = findSourceMapForGoFile(goFilePath);
    if (!mapPath) {
        return undefined;
    }

    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    const cwd = workspaceFolder?.uri.fsPath || path.dirname(goFilePath);

    try {
        const output = await execNavigate(
            [
                'navigate',
                '--source-map', mapPath,
                '--file', goFilePath,
                '--line', String(position.line + 1),
                '--column', String(position.character + 1),
                '--reverse',
            ],
            cwd
        );

        const loc = parseLocationOutput(output);
        if (!loc) {
            return undefined;
        }

        return new vscode.Location(
            vscode.Uri.file(loc.file),
            new vscode.Position(loc.line - 1, loc.column - 1)
        );
    } catch {
        return undefined;
    }
}

/**
 * Find the go.mod file closest to a given file path by walking upward.
 */
function findGoModDir(filePath: string): string | undefined {
    let dir = path.dirname(filePath);
    for (let i = 0; i < 20; i++) {
        if (fs.existsSync(path.join(dir, 'go.mod'))) {
            return dir;
        }
        const parent = path.dirname(dir);
        if (parent === dir) {
            break;
        }
        dir = parent;
    }
    return undefined;
}

/**
 * Resolve a Go import path to a directory on disk.
 * Supports:
 * 1. Relative to go.mod module path (local packages)
 * 2. Standard library (via GOROOT)
 * 3. Module cache (via GOPATH/pkg/mod)
 */
function resolveImportPath(importPath: string, documentPath: string): string | undefined {
    const goModDir = findGoModDir(documentPath);

    if (goModDir) {
        // Read go.mod to find module path
        try {
            const goModContent = fs.readFileSync(path.join(goModDir, 'go.mod'), 'utf-8');
            const moduleMatch = goModContent.match(/^module\s+(.+)$/m);
            if (moduleMatch) {
                const modulePath = moduleMatch[1].trim();
                if (importPath.startsWith(modulePath + '/')) {
                    // Local package
                    const relPath = importPath.substring(modulePath.length + 1);
                    const localDir = path.join(goModDir, ...relPath.split('/'));
                    if (fs.existsSync(localDir)) {
                        return localDir;
                    }
                }
            }
        } catch {
            // Ignore read errors
        }
    }

    // Try standard library via GOROOT
    try {
        const gorootResult = cp.execFileSync('go', ['env', 'GOROOT'], { timeout: 5000 }).toString().trim();
        if (gorootResult) {
            const stdlibDir = path.join(gorootResult, 'src', ...importPath.split('/'));
            if (fs.existsSync(stdlibDir)) {
                return stdlibDir;
            }
        }
    } catch {
        // Ignore errors
    }

    // Try module cache via GOPATH
    try {
        const gopathResult = cp.execFileSync('go', ['env', 'GOPATH'], { timeout: 5000 }).toString().trim();
        if (gopathResult) {
            const modCacheDir = path.join(gopathResult, 'pkg', 'mod');
            if (fs.existsSync(modCacheDir)) {
                // Module cache paths use the format: module@version/subpath
                // For simplicity, do a best-effort search
                const parts = importPath.split('/');
                for (let i = parts.length; i >= 1; i--) {
                    const prefix = parts.slice(0, i).join('/');
                    const subpath = parts.slice(i).join('/');
                    const cacheBase = path.join(modCacheDir, prefix);
                    try {
                        // Look for versioned directory
                        const parent = path.dirname(cacheBase);
                        const basename = path.basename(cacheBase);
                        if (fs.existsSync(parent)) {
                            const entries = fs.readdirSync(parent);
                            for (const entry of entries) {
                                if (entry.startsWith(basename + '@')) {
                                    const fullPath = subpath
                                        ? path.join(parent, entry, ...subpath.split('/'))
                                        : path.join(parent, entry);
                                    if (fs.existsSync(fullPath)) {
                                        return fullPath;
                                    }
                                }
                            }
                        }
                    } catch {
                        // Continue
                    }
                }
            }
        }
    } catch {
        // Ignore errors
    }

    return undefined;
}

/**
 * Find the first .go or .gp file in a directory that exports a given symbol.
 */
function findSymbolInDir(dirPath: string, symbolName: string): { file: string; line: number; col: number } | undefined {
    try {
        const entries = fs.readdirSync(dirPath);
        for (const entry of entries) {
            const ext = path.extname(entry);
            if (ext !== '.go' && ext !== '.gp') {
                continue;
            }
            if (entry.endsWith('_test.go')) {
                continue;
            }
            const filePath = path.join(dirPath, entry);
            const content = fs.readFileSync(filePath, 'utf-8');
            const lines = content.split('\n');
            const pattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|func|struct|enum|type|const|var|let)\\s+${symbolName}\\b`);
            const methodPattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|func)(?:\\s*\\([^)]+\\))?\\s+${symbolName}\\b`);
            for (let i = 0; i < lines.length; i++) {
                if (pattern.test(lines[i]) || methodPattern.test(lines[i])) {
                    const col = lines[i].indexOf(symbolName);
                    return { file: filePath, line: i, col: col > -1 ? col : 0 };
                }
            }
        }
    } catch {
        // Ignore errors
    }
    return undefined;
}

export class GoplusDefinitionProvider implements vscode.DefinitionProvider {
    async provideDefinition(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Definition | undefined> {
        const lineText = document.lineAt(position.line).text;

        // 1. Handle import path navigation: import "some/path"
        const importMatch = lineText.match(/^\s*import\s+(?:\w+\s+)?"([^"]+)"/);
        if (importMatch) {
            const importPath = importMatch[1];
            // Check if cursor is within the import string
            const quoteStart = lineText.indexOf('"');
            const quoteEnd = lineText.lastIndexOf('"');
            if (position.character >= quoteStart && position.character <= quoteEnd) {
                const resolved = resolveImportPath(importPath, document.uri.fsPath);
                if (resolved && fs.existsSync(resolved)) {
                    // Find the first .gp or .go file in the package directory
                    try {
                        const entries = fs.readdirSync(resolved);
                        for (const entry of entries) {
                            const ext = path.extname(entry);
                            if ((ext === '.gp' || ext === '.go') && !entry.endsWith('_test.go')) {
                                const filePath = path.join(resolved, entry);
                                return new vscode.Location(
                                    vscode.Uri.file(filePath),
                                    new vscode.Position(0, 0)
                                );
                            }
                        }
                    } catch {
                        // Ignore errors
                    }
                }
                return undefined;
            }
        }

        // 2. Handle dot-qualified identifiers: pkg.Symbol
        // Try to get a word range that includes dots
        const dotRange = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*\.[a-zA-Z_][a-zA-Z0-9_]*/);
        if (dotRange) {
            const fullWord = document.getText(dotRange);
            const dotIndex = fullWord.indexOf('.');
            const pkgName = fullWord.substring(0, dotIndex);
            const symbolName = fullWord.substring(dotIndex + 1);

            // Check if cursor is on the symbol part (after the dot)
            const dotCharPos = dotRange.start.character + dotIndex;
            if (position.character > dotCharPos) {
                // Cursor is on the symbol part — find the symbol in the package
                // First resolve the package from imports
                const importPath = this.findImportForPackage(document, pkgName);
                if (importPath) {
                    const resolved = resolveImportPath(importPath, document.uri.fsPath);
                    if (resolved) {
                        const result = findSymbolInDir(resolved, symbolName);
                        if (result) {
                            return new vscode.Location(
                                vscode.Uri.file(result.file),
                                new vscode.Position(result.line, result.col)
                            );
                        }
                    }
                }
            } else {
                // Cursor is on the package part — navigate to the import
                const importPath = this.findImportForPackage(document, pkgName);
                if (importPath) {
                    const resolved = resolveImportPath(importPath, document.uri.fsPath);
                    if (resolved && fs.existsSync(resolved)) {
                        try {
                            const entries = fs.readdirSync(resolved);
                            for (const entry of entries) {
                                const ext = path.extname(entry);
                                if ((ext === '.gp' || ext === '.go') && !entry.endsWith('_test.go')) {
                                    return new vscode.Location(
                                        vscode.Uri.file(path.join(resolved, entry)),
                                        new vscode.Position(0, 0)
                                    );
                                }
                            }
                        } catch {
                            // Ignore errors
                        }
                    }
                }
            }
        }

        // 3. Handle simple identifiers
        const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*/);
        if (!wordRange) {
            return undefined;
        }
        const word = document.getText(wordRange);

        // Search across workspace .gp AND .go files
        const uris = await vscode.workspace.findFiles('{**/*.gp,**/*.go}', '{**/node_modules/**,**/vendor/**}');
        // Add currently open documents that might not be saved
        const docs = new Set(vscode.workspace.textDocuments.filter(d => d.languageId === 'goplus' || d.languageId === 'go'));
        for (const uri of uris) {
            const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
            if (!openDoc) {
                try {
                    const doc = await vscode.workspace.openTextDocument(uri);
                    docs.add(doc);
                } catch {
                    // Ignore errors opening files
                }
            }
        }

        const pattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|func|struct|enum|type|const|var|let)\\s+${word}\\b`);
        const methodPattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|func)(?:\\s*\\([^)]+\\))?\\s+${word}\\b`);

        for (const doc of docs) {
            for (let i = 0; i < doc.lineCount; i++) {
                const text = doc.lineAt(i).text;
                if (pattern.test(text) || methodPattern.test(text)) {
                    // Found a definition
                    const matchIndex = text.indexOf(word);
                    return new vscode.Location(
                        doc.uri,
                        new vscode.Position(i, matchIndex > -1 ? matchIndex : 0)
                    );
                }
            }
        }

        return undefined;
    }

    /**
     * Find the import path for a given package alias in the document.
     */
    private findImportForPackage(document: vscode.TextDocument, pkgName: string): string | undefined {
        for (let i = 0; i < document.lineCount; i++) {
            const line = document.lineAt(i).text;
            // import alias "path"
            const aliasMatch = line.match(/^\s*import\s+(\w+)\s+"([^"]+)"/);
            if (aliasMatch && aliasMatch[1] === pkgName) {
                return aliasMatch[2];
            }
            // import "path" — package name is last segment of path
            const simpleMatch = line.match(/^\s*import\s+"([^"]+)"/);
            if (simpleMatch) {
                const importPath = simpleMatch[1];
                const lastSegment = importPath.split('/').pop();
                if (lastSegment === pkgName) {
                    return importPath;
                }
            }
        }
        return undefined;
    }
}
