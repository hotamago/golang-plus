import * as vscode from 'vscode';
export declare function inferTypeOfVariable(document: vscode.TextDocument, position: vscode.Position, variableName: string): Promise<string | undefined>;
export declare function inferTypeFromExpression(document: vscode.TextDocument, expr: string): Promise<string | undefined>;
//# sourceMappingURL=typeInference.d.ts.map