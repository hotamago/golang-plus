import * as vscode from 'vscode';
export declare class GoplusCompletionProvider implements vscode.CompletionItemProvider {
    provideCompletionItems(document: vscode.TextDocument, position: vscode.Position, token: vscode.CancellationToken, context: vscode.CompletionContext): vscode.ProviderResult<vscode.CompletionItem[] | vscode.CompletionList>;
    private createDeriveCompletion;
    private createDecoratorCompletion;
}
//# sourceMappingURL=completion.d.ts.map