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
exports.GoplusFormattingProvider = void 0;
const vscode = __importStar(require("vscode"));
const cp = __importStar(require("child_process"));
const path = __importStar(require("path"));
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
class GoplusFormattingProvider {
    provideDocumentFormattingEdits(document, _options, _token) {
        return new Promise((resolve, reject) => {
            const binary = getGoplusBinary();
            const filePath = document.uri.fsPath;
            const cwd = getCwd(document);
            cp.execFile(binary, ['fmt', '--stdout', filePath], { cwd, timeout: 15000 }, (err, stdout, stderr) => {
                if (err) {
                    const errorMessage = stderr || err.message;
                    vscode.window.showWarningMessage(`GoPlus format failed: ${errorMessage}`);
                    resolve([]);
                    return;
                }
                if (!stdout || stdout.trim().length === 0) {
                    resolve([]);
                    return;
                }
                const fullRange = new vscode.Range(document.lineAt(0).range.start, document.lineAt(document.lineCount - 1).range.end);
                resolve([vscode.TextEdit.replace(fullRange, stdout)]);
            });
        });
    }
}
exports.GoplusFormattingProvider = GoplusFormattingProvider;
//# sourceMappingURL=formatter.js.map