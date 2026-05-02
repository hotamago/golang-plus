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
exports.runInTerminal = runInTerminal;
exports.buildFile = buildFile;
exports.runFile = runFile;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
function runInTerminal(document, commandArg) {
    const config = vscode.workspace.getConfiguration('goplus');
    const binary = config.get('binaryPath', 'goplus');
    let terminal = vscode.window.terminals.find(t => t.name === 'GoPlus');
    if (!terminal) {
        terminal = vscode.window.createTerminal('GoPlus');
    }
    terminal.show();
    const filePath = document.uri.fsPath;
    let cwd = path.dirname(filePath);
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (workspaceFolder) {
        cwd = workspaceFolder.uri.fsPath;
    }
    // Convert to relatively clean paths or just use absolute
    const cmd = `${binary} ${commandArg} "${filePath}"`;
    terminal.sendText(`cd "${cwd}"`);
    terminal.sendText(cmd);
}
async function buildFile(document) {
    runInTerminal(document, 'build');
}
async function runFile(document) {
    runInTerminal(document, 'run');
}
//# sourceMappingURL=runner.js.map