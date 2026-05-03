import * as vscode from 'vscode';
export declare class GoplusHoverProvider implements vscode.HoverProvider {
    private findDefinitionAndHover;
    provideHover(document: vscode.TextDocument, position: vscode.Position, _token: vscode.CancellationToken): Promise<vscode.Hover | undefined>;
    private hoverDecorator;
    private hoverImportPath;
    private findImportForPackage;
    private findGoModDir;
    private execGoDoc;
}
//# sourceMappingURL=hover.d.ts.map