import * as vscode from 'vscode';

export class GoplusDocumentSymbolProvider implements vscode.DocumentSymbolProvider {
    public provideDocumentSymbols(
        document: vscode.TextDocument,
        token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DocumentSymbol[]> {
        const symbols: vscode.DocumentSymbol[] = [];
        const text = document.getText();
        const lines = text.split('\n');

        const regex = /^(?:pub\s+)?(?:fn|struct|enum|interface|type)\s+([a-zA-Z_]\w*)/;

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const match = line.match(regex);
            if (match) {
                const name = match[1];
                let kind = vscode.SymbolKind.Function;
                if (line.includes('struct ')) kind = vscode.SymbolKind.Struct;
                else if (line.includes('enum ')) kind = vscode.SymbolKind.Enum;
                else if (line.includes('interface ')) kind = vscode.SymbolKind.Interface;
                else if (line.includes('type ')) kind = vscode.SymbolKind.Class;

                // Simple range for the declaration line
                const range = new vscode.Range(i, 0, i, line.length);
                const nameStart = match.index! + match[0].length - name.length;
                const selectionRange = new vscode.Range(i, nameStart, i, nameStart + name.length);

                const symbol = new vscode.DocumentSymbol(
                    name,
                    '',
                    kind,
                    range,
                    selectionRange
                );
                symbols.push(symbol);
            }
        }
        return symbols;
    }
}
