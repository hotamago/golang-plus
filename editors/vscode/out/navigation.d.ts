import * as vscode from 'vscode';
/**
 * DefinitionProvider for .gp files: navigates to the generated .go location.
 */
export declare class GpToGoDefinitionProvider implements vscode.DefinitionProvider {
    provideDefinition(document: vscode.TextDocument, position: vscode.Position, _token: vscode.CancellationToken): Promise<vscode.Definition | undefined>;
}
/**
 * DefinitionProvider for generated .go files: navigates back to the .gp source.
 */
export declare class GoToGpDefinitionProvider implements vscode.DefinitionProvider {
    provideDefinition(document: vscode.TextDocument, position: vscode.Position, _token: vscode.CancellationToken): Promise<vscode.Definition | undefined>;
}
//# sourceMappingURL=navigation.d.ts.map