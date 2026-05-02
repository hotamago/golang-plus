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

/**
 * DefinitionProvider for .gp files: navigates to the generated .go location.
 */
export class GpToGoDefinitionProvider implements vscode.DefinitionProvider {
    async provideDefinition(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Definition | undefined> {
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
}

/**
 * DefinitionProvider for generated .go files: navigates back to the .gp source.
 */
export class GoToGpDefinitionProvider implements vscode.DefinitionProvider {
    async provideDefinition(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Definition | undefined> {
        const goFilePath = document.uri.fsPath;

        // Only activate for generated Go files
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
}
