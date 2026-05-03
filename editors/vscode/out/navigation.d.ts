import * as vscode from 'vscode';
export declare function navigateToGo(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Location | undefined>;
export declare function navigateToGp(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Location | undefined>;
export declare class GoplusDefinitionProvider implements vscode.DefinitionProvider {
    provideDefinition(document: vscode.TextDocument, position: vscode.Position, _token: vscode.CancellationToken): Promise<vscode.Definition | undefined>;
    /**
     * Find the import path for a given package alias in the document.
     */
    private findImportForPackage;
}
//# sourceMappingURL=navigation.d.ts.map