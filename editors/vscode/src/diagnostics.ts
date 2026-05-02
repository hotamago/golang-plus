import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';

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

interface DiagnosticsOutput {
    diagnostics: GoplusDiagnosticJson[];
}

function severityToVscode(sev: string): vscode.DiagnosticSeverity {
    switch (sev) {
        case 'error': return vscode.DiagnosticSeverity.Error;
        case 'warning': return vscode.DiagnosticSeverity.Warning;
        case 'info': return vscode.DiagnosticSeverity.Information;
        case 'hint': return vscode.DiagnosticSeverity.Hint;
        default: return vscode.DiagnosticSeverity.Error;
    }
}

function parseDiagnosticsJson(raw: string): GoplusDiagnosticJson[] {
    try {
        // The output may contain non-JSON lines (e.g. "no lint warnings").
        // Find the JSON object in the output.
        const jsonStart = raw.indexOf('{');
        if (jsonStart === -1) {
            return [];
        }
        const jsonStr = raw.substring(jsonStart);
        const parsed: DiagnosticsOutput = JSON.parse(jsonStr);
        return parsed.diagnostics || [];
    } catch {
        return [];
    }
}

function toDiagnostics(items: GoplusDiagnosticJson[]): vscode.Diagnostic[] {
    return items.map(item => {
        const range = item.span
            ? new vscode.Range(
                  Math.max(0, item.span.line - 1),
                  Math.max(0, item.span.column - 1),
                  Math.max(0, item.span.endLine - 1),
                  Math.max(0, item.span.endColumn - 1)
              )
            : new vscode.Range(0, 0, 0, 0);

        const diag = new vscode.Diagnostic(
            range,
            item.message,
            severityToVscode(item.severity)
        );
        diag.code = item.code;
        diag.source = 'goplus';

        if (item.hint) {
            diag.relatedInformation = [
                new vscode.DiagnosticRelatedInformation(
                    new vscode.Location(vscode.Uri.file(item.path), range),
                    `hint: ${item.hint}`
                ),
            ];
        }

        return diag;
    });
}

function getGoplusBinary(): string {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get<string>('binaryPath', 'goplus');
}

function getCwd(document: vscode.TextDocument): string {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (workspaceFolder) {
        return workspaceFolder.uri.fsPath;
    }
    return path.dirname(document.uri.fsPath);
}

function execGoplus(args: string[], cwd: string): Promise<string> {
    const binary = getGoplusBinary();
    return new Promise((resolve, reject) => {
        cp.execFile(binary, args, { cwd, timeout: 30000 }, (err, stdout, stderr) => {
            // goplus may exit with non-zero for diagnostics, which is expected.
            const output = (stdout || '') + (stderr || '');
            resolve(output);
        });
    });
}

export async function runCheck(document: vscode.TextDocument): Promise<vscode.Diagnostic[]> {
    const filePath = document.uri.fsPath;
    const cwd = getCwd(document);
    const output = await execGoplus(['check', filePath, '--diagnostic-format', 'json'], cwd);
    const items = parseDiagnosticsJson(output);
    return toDiagnostics(items);
}

export async function runLint(document: vscode.TextDocument): Promise<vscode.Diagnostic[]> {
    const filePath = document.uri.fsPath;
    const cwd = getCwd(document);
    const output = await execGoplus(['lint', filePath, '--diagnostic-format', 'json'], cwd);
    const items = parseDiagnosticsJson(output);
    return toDiagnostics(items);
}

export async function runAllDiagnostics(
    document: vscode.TextDocument,
    collection: vscode.DiagnosticCollection
): Promise<void> {
    const config = vscode.workspace.getConfiguration('goplus');
    const doCheck = config.get<boolean>('checkOnSave', true);
    const doLint = config.get<boolean>('lintOnSave', true);

    const allDiags: vscode.Diagnostic[] = [];

    if (doCheck) {
        try {
            const checkDiags = await runCheck(document);
            allDiags.push(...checkDiags);
        } catch {
            // Binary not found or other error — silently ignore
        }
    }

    if (doLint) {
        try {
            const lintDiags = await runLint(document);
            allDiags.push(...lintDiags);
        } catch {
            // Binary not found or other error — silently ignore
        }
    }

    collection.set(document.uri, allDiags);
}
