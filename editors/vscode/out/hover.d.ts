import * as vscode from 'vscode';
export declare class GoplusHoverProvider implements vscode.HoverProvider {
    private findDefinitionAndHover;
    provideHover(document: vscode.TextDocument, position: vscode.Position, _token: vscode.CancellationToken): Promise<vscode.Hover | undefined>;
    private hoverDecorator;
}
//# sourceMappingURL=hover.d.ts.map