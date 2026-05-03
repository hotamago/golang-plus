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
exports.GoplusInlayHintsProvider = void 0;
const vscode = __importStar(require("vscode"));
class GoplusInlayHintsProvider {
    provideInlayHints(document, range, _token) {
        const hints = [];
        for (let i = range.start.line; i <= range.end.line; i++) {
            const line = document.lineAt(i);
            const text = line.text;
            // Simple heuristic to find variable declarations: `let x = ...` or `x := ...`
            // and guess their type based on the right-hand side.
            const letMatch = text.match(/^\s*(?:let|var|const)\s+([a-zA-Z_]\w*)\s*=\s*(.+)$/);
            const walrusMatch = text.match(/^\s*([a-zA-Z_]\w*)\s*:=\s*(.+)$/);
            let varName = '';
            let rhs = '';
            let varIndex = -1;
            if (letMatch) {
                varName = letMatch[1];
                rhs = letMatch[2];
                varIndex = text.indexOf(varName) + varName.length;
            }
            else if (walrusMatch) {
                varName = walrusMatch[1];
                rhs = walrusMatch[2];
                varIndex = text.indexOf(varName) + varName.length;
            }
            if (varName && rhs && varIndex !== -1) {
                // Try to infer type
                let inferredType = '';
                rhs = rhs.trim();
                if (/^".*"$/.test(rhs) || /^`.*`$/.test(rhs)) {
                    inferredType = 'string';
                }
                else if (/^\d+\.\d+$/.test(rhs)) {
                    inferredType = 'float64';
                }
                else if (/^\d+$/.test(rhs)) {
                    inferredType = 'int';
                }
                else if (rhs === 'true' || rhs === 'false') {
                    inferredType = 'bool';
                }
                else if (rhs.startsWith('map[')) {
                    // map[string]int{...}
                    const mapMatch = rhs.match(/^(map\[.+\][a-zA-Z0-9_.*]+)/);
                    if (mapMatch)
                        inferredType = mapMatch[1];
                }
                else if (rhs.startsWith('[]')) {
                    const sliceMatch = rhs.match(/^(\[\].+?)\s*\{/);
                    if (sliceMatch)
                        inferredType = sliceMatch[1];
                }
                else if (/^[A-Z][a-zA-Z0-9_]*\s*\{/.test(rhs)) {
                    // Account{...} -> Account
                    const structMatch = rhs.match(/^([A-Z][a-zA-Z0-9_]*)\s*\{/);
                    if (structMatch)
                        inferredType = structMatch[1];
                }
                else if (/^[a-zA-Z_]\w*\s*\(/.test(rhs)) {
                    // Function call, we can't easily know the return type without a real typechecker.
                    // But we can check for some common ones or leave it empty.
                    const fnMatch = rhs.match(/^([a-zA-Z_]\w*)\s*\(/);
                    if (fnMatch) {
                        const fnName = fnMatch[1];
                        if (fnName === 'make') {
                            const makeMatch = rhs.match(/make\s*\(\s*([^,]+)/);
                            if (makeMatch)
                                inferredType = makeMatch[1].trim();
                        }
                        else if (fnName === 'append') {
                            // append returns the same type as the first argument, hard to infer exactly here
                        }
                    }
                }
                if (inferredType) {
                    const position = new vscode.Position(i, varIndex);
                    const hint = new vscode.InlayHint(position, `: ${inferredType}`, vscode.InlayHintKind.Type);
                    hint.paddingLeft = true;
                    hints.push(hint);
                }
            }
            // Handle `for idx, value := range values`
            const forRangeMatch = text.match(/^\s*for\s+([a-zA-Z_]\w*)\s*,\s*([a-zA-Z_]\w*)\s*:=\s*range\s+(.+)$/);
            if (forRangeMatch) {
                const idxName = forRangeMatch[1];
                const valName = forRangeMatch[2];
                const idxPos = text.indexOf(idxName) + idxName.length;
                const valPos = text.indexOf(valName, idxPos) + valName.length;
                // Usually idx is int, unless it's a map (then it's key type). We'll keep it simple.
                if (idxName !== '_') {
                    const hintIdx = new vscode.InlayHint(new vscode.Position(i, idxPos), `: int`, vscode.InlayHintKind.Type);
                    hintIdx.paddingLeft = true;
                    hints.push(hintIdx);
                }
            }
        }
        return hints;
    }
}
exports.GoplusInlayHintsProvider = GoplusInlayHintsProvider;
//# sourceMappingURL=inlayHints.js.map