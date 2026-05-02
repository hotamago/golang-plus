"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.GoplusDocumentSymbolProvider = void 0;
const vscode = __importStar(require("vscode"));
class GoplusDocumentSymbolProvider {
    provideDocumentSymbols(document, token) {
        const symbols = [];
        const text = document.getText();
        const lines = text.split('\n');
        const regex = /^(?:pub\s+)?(?:fn|struct|enum|interface|type)\s+([a-zA-Z_]\w*)/;
        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const match = line.match(regex);
            if (match) {
                const name = match[1];
                let kind = vscode.SymbolKind.Function;
                if (line.includes('struct '))
                    kind = vscode.SymbolKind.Struct;
                else if (line.includes('enum '))
                    kind = vscode.SymbolKind.Enum;
                else if (line.includes('interface '))
                    kind = vscode.SymbolKind.Interface;
                else if (line.includes('type '))
                    kind = vscode.SymbolKind.Class;
                // Simple range for the declaration line
                const range = new vscode.Range(i, 0, i, line.length);
                const nameStart = match.index + match[0].length - name.length;
                const selectionRange = new vscode.Range(i, nameStart, i, nameStart + name.length);
                const symbol = new vscode.DocumentSymbol(name, '', kind, range, selectionRange);
                symbols.push(symbol);
            }
        }
        return symbols;
    }
}
exports.GoplusDocumentSymbolProvider = GoplusDocumentSymbolProvider;
//# sourceMappingURL=symbols.js.map