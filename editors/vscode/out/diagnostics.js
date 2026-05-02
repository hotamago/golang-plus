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
exports.toDiagnostics = toDiagnostics;
exports.runCheck = runCheck;
exports.runLint = runLint;
exports.runAllDiagnostics = runAllDiagnostics;
const vscode = __importStar(require("vscode"));
const cp = __importStar(require("child_process"));
const path = __importStar(require("path"));
function severityToVscode(sev) {
    switch (sev) {
        case 'error': return vscode.DiagnosticSeverity.Error;
        case 'warning': return vscode.DiagnosticSeverity.Warning;
        case 'info': return vscode.DiagnosticSeverity.Information;
        case 'hint': return vscode.DiagnosticSeverity.Hint;
        default: return vscode.DiagnosticSeverity.Error;
    }
}
function parseDiagnosticsJson(raw) {
    const lines = raw.split('\n');
    const allDiagnostics = [];
    for (const line of lines) {
        const jsonStart = line.indexOf('{');
        if (jsonStart === -1)
            continue;
        try {
            const jsonStr = line.substring(jsonStart);
            const parsed = JSON.parse(jsonStr);
            if (parsed && Array.isArray(parsed.diagnostics)) {
                allDiagnostics.push(...parsed.diagnostics);
            }
            else if (parsed && parsed.path && parsed.message) {
                // Handle fallback where single diagnostic is printed
                allDiagnostics.push(parsed);
            }
        }
        catch {
            // Ignore parse errors on this line
        }
    }
    return allDiagnostics;
}
function normalizePath(p) {
    if (p.startsWith('\\\\?\\')) {
        return p.substring(4);
    }
    return p;
}
function toDiagnostics(items) {
    const map = new Map();
    for (const item of items) {
        const range = item.span
            ? new vscode.Range(Math.max(0, item.span.line - 1), Math.max(0, item.span.column - 1), Math.max(0, item.span.endLine - 1), Math.max(0, item.span.endColumn - 1))
            : new vscode.Range(0, 0, 0, 0);
        let message = item.message;
        if (item.hint) {
            message += `\n\nHint: ${item.hint}`;
        }
        const diag = new vscode.Diagnostic(range, message, severityToVscode(item.severity));
        diag.code = item.code;
        diag.source = 'goplus';
        const normPath = normalizePath(item.path);
        if (item.hint) {
            diag.relatedInformation = [
                new vscode.DiagnosticRelatedInformation(new vscode.Location(vscode.Uri.file(normPath), range), `hint: ${item.hint}`),
            ];
        }
        const uriStr = vscode.Uri.file(normPath).toString();
        if (!map.has(uriStr)) {
            map.set(uriStr, []);
        }
        map.get(uriStr).push(diag);
    }
    return map;
}
function getGoplusBinary() {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get('binaryPath', 'goplus');
}
function getCwd(document) {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (workspaceFolder) {
        return workspaceFolder.uri.fsPath;
    }
    return path.dirname(document.uri.fsPath);
}
function execGoplus(args, cwd) {
    const binary = getGoplusBinary();
    return new Promise((resolve, reject) => {
        cp.execFile(binary, args, { cwd, timeout: 30000, maxBuffer: 1024 * 1024 * 5 }, (err, stdout, stderr) => {
            if (err && err.code === 'ENOENT') {
                return reject(new Error(`GoPlus binary not found: ${binary}`));
            }
            const output = (stdout || '') + '\n' + (stderr || '');
            resolve(output);
        });
    });
}
async function runCheck(document) {
    const filePath = document.uri.fsPath;
    const cwd = getCwd(document);
    const output = await execGoplus(['check', filePath, '--diagnostic-format', 'json'], cwd);
    return parseDiagnosticsJson(output);
}
async function runLint(document) {
    const filePath = document.uri.fsPath;
    const cwd = getCwd(document);
    const output = await execGoplus(['lint', filePath, '--diagnostic-format', 'json'], cwd);
    return parseDiagnosticsJson(output);
}
async function runAllDiagnostics(document, collection) {
    const config = vscode.workspace.getConfiguration('goplus');
    const doCheck = config.get('checkOnSave', true);
    const doLint = config.get('lintOnSave', true);
    const allItems = [];
    if (doCheck) {
        try {
            const checkItems = await runCheck(document);
            allItems.push(...checkItems);
        }
        catch {
            // Binary not found or other error — silently ignore
        }
    }
    if (doLint) {
        try {
            const lintItems = await runLint(document);
            allItems.push(...lintItems);
        }
        catch {
            // Binary not found or other error — silently ignore
        }
    }
    // Clear previous diagnostics for this document first
    collection.set(document.uri, []);
    const grouped = toDiagnostics(allItems);
    // We should also ensure that if there are NO diagnostics for the active document, it gets cleared.
    // Setting it to [] above handles it.
    for (const [uriStr, diags] of grouped.entries()) {
        collection.set(vscode.Uri.parse(uriStr), diags);
    }
}
//# sourceMappingURL=diagnostics.js.map