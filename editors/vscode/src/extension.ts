import * as vscode from 'vscode';
import { runAllDiagnostics, runCheck, runLint, toDiagnostics } from './diagnostics';
import { GoplusFormattingProvider } from './formatter';
import { navigateToGo, navigateToGp, GoplusDefinitionProvider } from './navigation';
import { GoplusHoverProvider } from './hover';
import { GoplusDocumentSymbolProvider } from './symbols';
import { buildFile, runFile } from './runner';
import { GoplusCompletionProvider } from './completion';

const GP_SELECTOR: vscode.DocumentSelector = { language: 'goplus', scheme: 'file' };

let diagnosticCollection: vscode.DiagnosticCollection;

export function activate(context: vscode.ExtensionContext): void {
    console.log('GoPlus extension activated');

    // --- Diagnostic collection ---
    diagnosticCollection = vscode.languages.createDiagnosticCollection('goplus');
    context.subscriptions.push(diagnosticCollection);

    // --- Run diagnostics on save ---
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(async (document) => {
            if (document.languageId !== 'goplus') {
                return;
            }
            await runAllDiagnostics(document, diagnosticCollection);
        })
    );

    // --- Run diagnostics on open ---
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(async (document) => {
            if (document.languageId !== 'goplus') {
                return;
            }
            await runAllDiagnostics(document, diagnosticCollection);
        })
    );

    // --- Clear diagnostics on close ---
    context.subscriptions.push(
        vscode.workspace.onDidCloseTextDocument((document) => {
            diagnosticCollection.delete(document.uri);
        })
    );

    // --- Run diagnostics for already-open .gp files ---
    for (const document of vscode.workspace.textDocuments) {
        if (document.languageId === 'goplus') {
            runAllDiagnostics(document, diagnosticCollection);
        }
    }

    // --- Format provider ---
    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(
            GP_SELECTOR,
            new GoplusFormattingProvider()
        )
    );

    // --- Navigation providers ---
    // Standard Go to Definition for .gp files -> jumps to .gp definition
    context.subscriptions.push(
        vscode.languages.registerDefinitionProvider(
            GP_SELECTOR,
            new GoplusDefinitionProvider()
        )
    );

    // --- Hover provider ---
    context.subscriptions.push(
        vscode.languages.registerHoverProvider(
            GP_SELECTOR,
            new GoplusHoverProvider()
        )
    );

    // --- Document Symbol Provider ---
    context.subscriptions.push(
        vscode.languages.registerDocumentSymbolProvider(
            GP_SELECTOR,
            new GoplusDocumentSymbolProvider()
        )
    );

    // --- Completion Provider ---
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            GP_SELECTOR,
            new GoplusCompletionProvider(),
            '@', // trigger for decorators
            '(', // trigger for derive kinds
            ':'  // trigger for enum variants (::)
        )
    );

    // --- Commands ---
    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.checkFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'goplus') {
                vscode.window.showWarningMessage('Open a .gp file to run GoPlus check.');
                return;
            }
            const items = await runCheck(editor.document);
            const grouped = toDiagnostics(items);
            
            diagnosticCollection.set(editor.document.uri, []);
            for (const [uriStr, diags] of grouped.entries()) {
                diagnosticCollection.set(vscode.Uri.parse(uriStr), diags);
            }
            if (items.length === 0) {
                vscode.window.showInformationMessage('GoPlus check: no issues found.');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.lintFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'goplus') {
                vscode.window.showWarningMessage('Open a .gp file to run GoPlus lint.');
                return;
            }
            const items = await runLint(editor.document);
            const grouped = toDiagnostics(items);
            
            diagnosticCollection.set(editor.document.uri, []);
            for (const [uriStr, diags] of grouped.entries()) {
                diagnosticCollection.set(vscode.Uri.parse(uriStr), diags);
            }
            if (items.length === 0) {
                vscode.window.showInformationMessage('GoPlus lint: no warnings found.');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.formatFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'goplus') {
                vscode.window.showWarningMessage('Open a .gp file to run GoPlus format.');
                return;
            }
            await vscode.commands.executeCommand('editor.action.formatDocument');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.navigateToGo', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'goplus') {
                vscode.window.showWarningMessage('Open a .gp file to navigate to generated Go.');
                return;
            }
            const loc = await navigateToGo(editor.document, editor.selection.active);
            if (loc) {
                const doc = await vscode.workspace.openTextDocument(loc.uri);
                await vscode.window.showTextDocument(doc, { selection: loc.range });
            } else {
                vscode.window.showWarningMessage('Could not find corresponding Go code.');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.navigateToGp', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('Open a generated Go file to navigate back to GoPlus source.');
                return;
            }
            const loc = await navigateToGp(editor.document, editor.selection.active);
            if (loc) {
                const doc = await vscode.workspace.openTextDocument(loc.uri);
                await vscode.window.showTextDocument(doc, { selection: loc.range });
            } else {
                vscode.window.showWarningMessage('Could not find corresponding GoPlus code.');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.buildFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'goplus') {
                vscode.window.showWarningMessage('Open a .gp file to build.');
                return;
            }
            await buildFile(editor.document);
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('goplus.runFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'goplus') {
                vscode.window.showWarningMessage('Open a .gp file to run.');
                return;
            }
            await runFile(editor.document);
        })
    );
}

export function deactivate(): void {
    if (diagnosticCollection) {
        diagnosticCollection.dispose();
    }
}
