import * as vscode from 'vscode';
export declare class GoplusCompletionProvider implements vscode.CompletionItemProvider {
    provideCompletionItems(document: vscode.TextDocument, position: vscode.Position, token: vscode.CancellationToken, context: vscode.CompletionContext): Promise<vscode.CompletionItem[] | vscode.CompletionList>;
    private getEnumVariants;
    private createDeriveCompletion;
    private createDecoratorCompletion;
}
//# sourceMappingURL=completion.d.ts.map