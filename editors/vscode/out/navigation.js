"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.GoToGpDefinitionProvider = exports.GpToGoDefinitionProvider = void 0;
const vscode = __importStar(require("vscode"));
const cp = __importStar(require("child_process"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
function getGoplusBinary() {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get('binaryPath', 'goplus');
}
function getOutDir() {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get('outDir', '.goplusgen');
}
/**
 * Find all `.go.map` files in the workspace's outDir.
 */
function findSourceMaps(workspaceRoot) {
    const outDir = path.resolve(workspaceRoot, getOutDir());
    const results = [];
    function walk(dir) {
        try {
            const entries = fs.readdirSync(dir, { withFileTypes: true });
            for (const entry of entries) {
                const fullPath = path.join(dir, entry.name);
                if (entry.isDirectory() && !entry.name.startsWith('.')) {
                    walk(fullPath);
                }
                else if (entry.isFile() && entry.name.endsWith('.go.map')) {
                    results.push(fullPath);
                }
            }
        }
        catch {
            // Directory may not exist
        }
    }
    walk(outDir);
    return results;
}
/**
 * Find the source map that contains a reference to a given .gp source file.
 */
function findSourceMapForGpFile(workspaceRoot, gpFilePath) {
    const maps = findSourceMaps(workspaceRoot);
    const normalizedGp = gpFilePath.replace(/\\/g, '/').toLowerCase();
    for (const mapPath of maps) {
        try {
            const content = fs.readFileSync(mapPath, 'utf-8');
            const map = JSON.parse(content);
            const hasSource = map.sources.some(s => s.replace(/\\/g, '/').toLowerCase().includes(normalizedGp.split('/').slice(-2).join('/')));
            if (hasSource) {
                return mapPath;
            }
        }
        catch {
            // Skip invalid maps
        }
    }
    return undefined;
}
/**
 * Find the source map for a generated .go file (look for .go.map next to it).
 */
function findSourceMapForGoFile(goFilePath) {
    const mapPath = goFilePath + '.map';
    if (fs.existsSync(mapPath)) {
        return mapPath;
    }
    return undefined;
}
function execNavigate(args, cwd) {
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
function parseLocationOutput(output) {
    // Output format: <file>:<line>:<column>
    const parts = output.split(':');
    if (parts.length < 3) {
        return undefined;
    }
    // Handle Windows paths with drive letter (e.g., D:\path:10:5)
    let file;
    let lineStr;
    let colStr;
    if (parts.length >= 4 && /^[a-zA-Z]$/.test(parts[0])) {
        // Windows path: D:\path\file.go:10:5
        file = parts[0] + ':' + parts[1];
        lineStr = parts[2];
        colStr = parts[3];
    }
    else {
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
class GpToGoDefinitionProvider {
    async provideDefinition(document, position, _token) {
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
            const output = await execNavigate([
                'navigate',
                '--source-map', mapPath,
                '--file', gpFilePath,
                '--line', String(position.line + 1),
                '--column', String(position.character + 1),
            ], workspaceFolder.uri.fsPath);
            const loc = parseLocationOutput(output);
            if (!loc) {
                return undefined;
            }
            return new vscode.Location(vscode.Uri.file(loc.file), new vscode.Position(loc.line - 1, loc.column - 1));
        }
        catch {
            return undefined;
        }
    }
}
exports.GpToGoDefinitionProvider = GpToGoDefinitionProvider;
/**
 * DefinitionProvider for generated .go files: navigates back to the .gp source.
 */
class GoToGpDefinitionProvider {
    async provideDefinition(document, position, _token) {
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
            const output = await execNavigate([
                'navigate',
                '--source-map', mapPath,
                '--file', goFilePath,
                '--line', String(position.line + 1),
                '--column', String(position.character + 1),
                '--reverse',
            ], cwd);
            const loc = parseLocationOutput(output);
            if (!loc) {
                return undefined;
            }
            return new vscode.Location(vscode.Uri.file(loc.file), new vscode.Position(loc.line - 1, loc.column - 1));
        }
        catch {
            return undefined;
        }
    }
}
exports.GoToGpDefinitionProvider = GoToGpDefinitionProvider;
//# sourceMappingURL=navigation.js.map