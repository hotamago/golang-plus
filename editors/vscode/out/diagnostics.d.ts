import * as vscode from 'vscode';
/** Shape of a single diagnostic from `goplus check/lint --diagnostic-format json`. */
export interface GoplusDiagnosticJson {
    path: string;
    code: string;
    severity: 'error' | 'warning' | 'info' | 'hint';
    message: string;
    span: {
        start: number;
        end: number;
        line: number;
        column: number;
        endLine: number;
        endColumn: number;
    } | null;
    hint: string | null;
    source: string;
}
export declare function runCheck(document: vscode.TextDocument): Promise<vscode.Diagnostic[]>;
export declare function runLint(document: vscode.TextDocument): Promise<vscode.Diagnostic[]>;
export declare function runAllDiagnostics(document: vscode.TextDocument, collection: vscode.DiagnosticCollection): Promise<void>;
//# sourceMappingURL=diagnostics.d.ts.map