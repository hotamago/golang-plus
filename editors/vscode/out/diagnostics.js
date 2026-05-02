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
    try {
        // The output may contain non-JSON lines (e.g. "no lint warnings").
        // Find the JSON object in the output.
        const jsonStart = raw.indexOf('{');
        if (jsonStart === -1) {
            return [];
        }
        const jsonStr = raw.substring(jsonStart);
        const parsed = JSON.parse(jsonStr);
        return parsed.diagnostics || [];
    }
    catch {
        return [];
    }
}
function toDiagnostics(items) {
    return items.map(item => {
        const range = item.span
            ? new vscode.Range(Math.max(0, item.span.line - 1), Math.max(0, item.span.column - 1), Math.max(0, item.span.endLine - 1), Math.max(0, item.span.endColumn - 1))
            : new vscode.Range(0, 0, 0, 0);
        const diag = new vscode.Diagnostic(range, item.message, severityToVscode(item.severity));
        diag.code = item.code;
        diag.source = 'goplus';
        if (item.hint) {
            diag.relatedInformation = [
                new vscode.DiagnosticRelatedInformation(new vscode.Location(vscode.Uri.file(item.path), range), `hint: ${item.hint}`),
            ];
        }
        return diag;
    });
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
        cp.execFile(binary, args, { cwd, timeout: 30000 }, (err, stdout, stderr) => {
            // goplus may exit with non-zero for diagnostics, which is expected.
            const output = (stdout || '') + (stderr || '');
            resolve(output);
        });
    });
}
async function runCheck(document) {
    const filePath = document.uri.fsPath;
    const cwd = getCwd(document);
    const output = await execGoplus(['check', filePath, '--diagnostic-format', 'json'], cwd);
    const items = parseDiagnosticsJson(output);
    return toDiagnostics(items);
}
async function runLint(document) {
    const filePath = document.uri.fsPath;
    const cwd = getCwd(document);
    const output = await execGoplus(['lint', filePath, '--diagnostic-format', 'json'], cwd);
    const items = parseDiagnosticsJson(output);
    return toDiagnostics(items);
}
async function runAllDiagnostics(document, collection) {
    const config = vscode.workspace.getConfiguration('goplus');
    const doCheck = config.get('checkOnSave', true);
    const doLint = config.get('lintOnSave', true);
    const allDiags = [];
    if (doCheck) {
        try {
            const checkDiags = await runCheck(document);
            allDiags.push(...checkDiags);
        }
        catch {
            // Binary not found or other error — silently ignore
        }
    }
    if (doLint) {
        try {
            const lintDiags = await runLint(document);
            allDiags.push(...lintDiags);
        }
        catch {
            // Binary not found or other error — silently ignore
        }
    }
    collection.set(document.uri, allDiags);
}
//# sourceMappingURL=diagnostics.js.map