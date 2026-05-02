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
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const diagnostics_1 = require("./diagnostics");
const formatter_1 = require("./formatter");
const navigation_1 = require("./navigation");
const hover_1 = require("./hover");
const symbols_1 = require("./symbols");
const runner_1 = require("./runner");
const GP_SELECTOR = { language: 'goplus', scheme: 'file' };
let diagnosticCollection;
function activate(context) {
    console.log('GoPlus extension activated');
    // --- Diagnostic collection ---
    diagnosticCollection = vscode.languages.createDiagnosticCollection('goplus');
    context.subscriptions.push(diagnosticCollection);
    // --- Run diagnostics on save ---
    context.subscriptions.push(vscode.workspace.onDidSaveTextDocument(async (document) => {
        if (document.languageId !== 'goplus') {
            return;
        }
        await (0, diagnostics_1.runAllDiagnostics)(document, diagnosticCollection);
    }));
    // --- Run diagnostics on open ---
    context.subscriptions.push(vscode.workspace.onDidOpenTextDocument(async (document) => {
        if (document.languageId !== 'goplus') {
            return;
        }
        await (0, diagnostics_1.runAllDiagnostics)(document, diagnosticCollection);
    }));
    // --- Clear diagnostics on close ---
    context.subscriptions.push(vscode.workspace.onDidCloseTextDocument((document) => {
        diagnosticCollection.delete(document.uri);
    }));
    // --- Run diagnostics for already-open .gp files ---
    for (const document of vscode.workspace.textDocuments) {
        if (document.languageId === 'goplus') {
            (0, diagnostics_1.runAllDiagnostics)(document, diagnosticCollection);
        }
    }
    // --- Format provider ---
    context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider(GP_SELECTOR, new formatter_1.GoplusFormattingProvider()));
    // --- Navigation providers ---
    // Standard Go to Definition for .gp files -> jumps to .gp definition
    context.subscriptions.push(vscode.languages.registerDefinitionProvider(GP_SELECTOR, new navigation_1.GoplusDefinitionProvider()));
    // --- Hover provider ---
    context.subscriptions.push(vscode.languages.registerHoverProvider(GP_SELECTOR, new hover_1.GoplusHoverProvider()));
    // --- Document Symbol Provider ---
    context.subscriptions.push(vscode.languages.registerDocumentSymbolProvider(GP_SELECTOR, new symbols_1.GoplusDocumentSymbolProvider()));
    // --- Commands ---
    context.subscriptions.push(vscode.commands.registerCommand('goplus.checkFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'goplus') {
            vscode.window.showWarningMessage('Open a .gp file to run GoPlus check.');
            return;
        }
        const diags = await (0, diagnostics_1.runCheck)(editor.document);
        diagnosticCollection.set(editor.document.uri, diags);
        if (diags.length === 0) {
            vscode.window.showInformationMessage('GoPlus check: no issues found.');
        }
    }));
    context.subscriptions.push(vscode.commands.registerCommand('goplus.lintFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'goplus') {
            vscode.window.showWarningMessage('Open a .gp file to run GoPlus lint.');
            return;
        }
        const diags = await (0, diagnostics_1.runLint)(editor.document);
        diagnosticCollection.set(editor.document.uri, diags);
        if (diags.length === 0) {
            vscode.window.showInformationMessage('GoPlus lint: no warnings found.');
        }
    }));
    context.subscriptions.push(vscode.commands.registerCommand('goplus.formatFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'goplus') {
            vscode.window.showWarningMessage('Open a .gp file to run GoPlus format.');
            return;
        }
        await vscode.commands.executeCommand('editor.action.formatDocument');
    }));
    context.subscriptions.push(vscode.commands.registerCommand('goplus.navigateToGo', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'goplus') {
            vscode.window.showWarningMessage('Open a .gp file to navigate to generated Go.');
            return;
        }
        const loc = await (0, navigation_1.navigateToGo)(editor.document, editor.selection.active);
        if (loc) {
            const doc = await vscode.workspace.openTextDocument(loc.uri);
            await vscode.window.showTextDocument(doc, { selection: loc.range });
        }
        else {
            vscode.window.showWarningMessage('Could not find corresponding Go code.');
        }
    }));
    context.subscriptions.push(vscode.commands.registerCommand('goplus.navigateToGp', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('Open a generated Go file to navigate back to GoPlus source.');
            return;
        }
        const loc = await (0, navigation_1.navigateToGp)(editor.document, editor.selection.active);
        if (loc) {
            const doc = await vscode.workspace.openTextDocument(loc.uri);
            await vscode.window.showTextDocument(doc, { selection: loc.range });
        }
        else {
            vscode.window.showWarningMessage('Could not find corresponding GoPlus code.');
        }
    }));
    context.subscriptions.push(vscode.commands.registerCommand('goplus.buildFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'goplus') {
            vscode.window.showWarningMessage('Open a .gp file to build.');
            return;
        }
        await (0, runner_1.buildFile)(editor.document);
    }));
    context.subscriptions.push(vscode.commands.registerCommand('goplus.runFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'goplus') {
            vscode.window.showWarningMessage('Open a .gp file to run.');
            return;
        }
        await (0, runner_1.runFile)(editor.document);
    }));
}
function deactivate() {
    if (diagnosticCollection) {
        diagnosticCollection.dispose();
    }
}
//# sourceMappingURL=extension.js.map