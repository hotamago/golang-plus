import * as vscode from 'vscode';

interface HoverEntry {
    pattern: RegExp;
    content: (match: RegExpMatchArray, lineText: string) => vscode.MarkdownString | undefined;
}

const DECORATOR_DOCS: Record<string, { signature: string; description: string; example: string }> = {
    'log': {
        signature: '@log',
        description: 'Wraps the function with automatic entry/exit logging. Prints function name and arguments on call, and return values on completion.',
        example: '@log\nfn readName() -> string! {\n    return "goplus"\n}',
    },
    'retry': {
        signature: '@retry(times: int[, backoff_ms: int])',
        description: 'Retries the function up to `times` attempts on error. Optional `backoff_ms` adds a delay between retries.',
        example: '@retry(3, 100)\nfn fetchData() -> string! {\n    return http.Get(url)?\n}',
    },
    'memoize': {
        signature: '@memoize',
        description: 'Caches function results by arguments. Subsequent calls with the same arguments return the cached result without re-executing the function body.',
        example: '@memoize\nfn fibonacci(n: int) -> int {\n    if n <= 1 { return n }\n    return fibonacci(n-1) + fibonacci(n-2)\n}',
    },
    'derive': {
        signature: '@derive(Kind1, Kind2, ...)',
        description: 'Generates method implementations for structs and enums at compile time.',
        example: '@derive(String, Debug, Equal)\nstruct User {\n    Name: string\n    Age: int\n}',
    },
};

const DERIVE_KIND_DOCS: Record<string, string> = {
    'String': '**String** — generates `func (t Type) String() string` for human-readable output via `fmt.Println`.',
    'Debug': '**Debug** — generates `func (t Type) Debug() string` with detailed field-level debug representation.',
    'Equal': '**Equal** — generates `func (t Type) Equal(other Type) bool` for structural equality comparison.',
    'JsonMarshal': '**JsonMarshal** — generates `func (t Type) MarshalJSON() ([]byte, error)` for JSON serialization.',
    'JsonUnmarshal': '**JsonUnmarshal** — generates `func (t *Type) UnmarshalJSON(data []byte) error` for JSON deserialization.',
};

export class GoplusHoverProvider implements vscode.HoverProvider {
    private async findDefinitionAndHover(document: vscode.TextDocument, position: vscode.Position, word: string): Promise<vscode.Hover | undefined> {
        // 1. Local scope search (upwards from current line)
        for (let i = position.line; i >= 0; i--) {
            const text = document.lineAt(i).text;
            
            // Check for local variable declaration
            const localPattern = new RegExp(`^\\s*(?:(?:let|var|const)\\s+${word}\\b|${word}\\s*:=)`);
            if (localPattern.test(text)) {
                const md = new vscode.MarkdownString();
                md.appendCodeblock(text.trim(), 'gp');
                return new vscode.Hover(md);
            }

            // Check for function parameter
            const fnPattern = /^\s*fn(?: \w+)?\s+\w+\((.*)\)/;
            const match = text.match(fnPattern);
            if (match) {
                const params = match[1];
                const paramPattern = new RegExp(`\\b${word}\\s*:`);
                if (paramPattern.test(params)) {
                    const typeMatch = params.match(new RegExp(`\\b${word}\\s*:\\s*([^,)]+)`));
                    const paramType = typeMatch ? typeMatch[1].trim() : 'unknown';
                    const md = new vscode.MarkdownString();
                    md.appendCodeblock(`(parameter) ${word}: ${paramType}`, 'gp');
                    return new vscode.Hover(md);
                }
                // Stop searching upwards if we hit a function definition boundary
                break;
            }
        }

        // 2. Global search across workspace .gp and .go files
        const uris = await vscode.workspace.findFiles('{**/*.gp,**/*.go}', '**/node_modules/**');
        const docs = new Set(vscode.workspace.textDocuments.filter(d => d.languageId === 'goplus' || d.languageId === 'go'));
        for (const uri of uris) {
            const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
            if (!openDoc) {
                try {
                    const doc = await vscode.workspace.openTextDocument(uri);
                    docs.add(doc);
                } catch {
                    // Ignore errors opening files
                }
            }
        }

        const pattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|func|struct|enum|type|const|var|let)\\s+${word}\\b`);
        const methodPattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|func)(?:\\s*\\([^)]+\\))?\\s+${word}\\b`);

        for (const doc of docs) {
            for (let i = 0; i < doc.lineCount; i++) {
                const text = doc.lineAt(i).text;
                if (pattern.test(text) || methodPattern.test(text)) {
                    // Found a definition! Extract the signature, possibly spanning multiple lines
                    let signature = text.trim();
                    let currentLine = i;
                    // If it's a block definition, read until '{' or we read too many lines
                    if (signature.startsWith('fn') || signature.startsWith('func') || signature.startsWith('struct') || signature.startsWith('enum')) {
                        while (!signature.includes('{') && currentLine < doc.lineCount - 1 && currentLine - i < 10) {
                            currentLine++;
                            const nextLine = doc.lineAt(currentLine).text.trim();
                            signature += '\n' + nextLine;
                        }
                        signature = signature.replace(/\s*\{$/, '');
                    }
                    
                    // Look for preceding comments
                    const comments: string[] = [];
                    let j = i - 1;
                    while (j >= 0) {
                        const prevLine = doc.lineAt(j).text.trim();
                        if (prevLine.startsWith('//')) {
                            comments.unshift(prevLine.substring(2).trim());
                            j--;
                        } else {
                            break;
                        }
                    }

                    const md = new vscode.MarkdownString();
                    md.appendCodeblock(signature, 'gp');
                    if (comments.length > 0) {
                        md.appendMarkdown('\n---\n' + comments.join('  \n'));
                    }
                    return new vscode.Hover(md);
                }
            }
        }

        return undefined;
    }

    async provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Hover | undefined> {
        const lineText = document.lineAt(position.line).text;
        
        // Skip hover if inside a string or comment
        let inString = false;
        let inChar = false;
        for (let i = 0; i < position.character; i++) {
            const c = lineText[i];
            const prev = i > 0 ? lineText[i - 1] : '';
            if (c === '"' && prev !== '\\' && !inChar) {
                inString = !inString;
            } else if (c === "'" && prev !== '\\' && !inString) {
                inChar = !inChar;
            } else if (c === '/' && prev === '/' && !inString && !inChar) {
                return undefined;
            }
        }
        if (inString || inChar) {
            return undefined;
        }

        const wordRange = document.getWordRangeAtPosition(position, /[@a-zA-Z_!?:][a-zA-Z0-9_.]*/);
        if (!wordRange) {
            return undefined;
        }

        const word = document.getText(wordRange);
        const charAtPos = lineText[position.character] || '';

        // Decorator hover: @name
        if (word.startsWith('@')) {
            const decoratorName = word.substring(1);
            return await this.hoverDecorator(document, position, decoratorName, lineText);
        }

        // Derive kind hover inside @derive(...)
        if (DERIVE_KIND_DOCS[word] && lineText.includes('@derive')) {
            const md = new vscode.MarkdownString();
            md.appendMarkdown(`### \`@derive(${word})\`\n\n`);
            md.appendMarkdown(DERIVE_KIND_DOCS[word]);
            return new vscode.Hover(md);
        }

        // ? operator hover
        if (charAtPos === '?') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `?` — Try Operator\n\n');
            md.appendMarkdown('Unwraps the result value or returns the error to the caller.\n\n');
            md.appendMarkdown('**Generated Go equivalent:**\n');
            md.appendCodeblock(
                'val, err := expr()\nif err != nil {\n    return ..., err\n}',
                'go'
            );
            return new vscode.Hover(md);
        }

        // ! error return type hover
        if (charAtPos === '!') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `!` — Error Return Type\n\n');
            md.appendMarkdown('Indicates the function returns an error alongside its value.\n\n');
            md.appendMarkdown('| GoPlus | Generated Go |\n|--------|-------------|\n');
            md.appendMarkdown('| `-> T!` | `(T, error)` |\n');
            md.appendMarkdown('| `-> !` | `error` (main becomes a wrapper) |\n');
            return new vscode.Hover(md);
        }

        // :: namespace operator
        if (word === '::' || lineText.substring(position.character, position.character + 2) === '::') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `::` — Enum Variant Access\n\n');
            md.appendMarkdown('Accesses a variant of a tagged or simple enum.\n\n');
            md.appendMarkdown('```gp\nStatus::Running\nResult<string>::Ok("value")\n```');
            return new vscode.Hover(md);
        }

        // self keyword
        if (word === 'self') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `self` — Receiver\n\n');
            md.appendMarkdown('Refers to the current instance of the `impl` target type.\n\n');
            md.appendMarkdown('- `fn method(self)` → value receiver `func (s Type)`\n');
            md.appendMarkdown('- `fn mut method(self)` → pointer receiver `func (s *Type)`\n');
            return new vscode.Hover(md);
        }

        // match keyword
        if (word === 'match') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `match` — Pattern Matching\n\n');
            md.appendMarkdown('Exhaustively matches enum variants. The compiler checks that all variants are covered.\n\n');
            md.appendMarkdown('```gp\nmatch result {\n    Ok(value) => "success: " + value,\n    Err(reason) => "error: " + reason,\n}\n```\n\n');
            md.appendMarkdown('Compiles to a Go `switch` statement.');
            return new vscode.Hover(md);
        }

        // fn keyword
        if (word === 'fn') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `fn` — Function Declaration\n\n');
            md.appendMarkdown('Declares a function with GoPlus syntax.\n\n');
            md.appendMarkdown('```gp\nfn name(param: Type) -> ReturnType {\n    // body\n}\n```\n\n');
            md.appendMarkdown('Inside `impl` blocks, use `fn mut` for pointer receivers.');
            return new vscode.Hover(md);
        }

        // enum keyword
        if (word === 'enum') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `enum` — Enum Declaration\n\n');
            md.appendMarkdown('Declares a sum type with simple or tagged (payload) variants.\n\n');
            md.appendMarkdown('```gp\nenum Result<T> {\n    Ok(T)\n    Err(string)\n}\n```\n\n');
            md.appendMarkdown('Use `@derive(String)` to add `String()` method.');
            return new vscode.Hover(md);
        }

        // struct keyword
        if (word === 'struct') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `struct` — Struct Declaration\n\n');
            md.appendMarkdown('Declares a product type with named fields.\n\n');
            md.appendMarkdown('```gp\nstruct User {\n    Name: string\n    Age: int\n}\n```');
            return new vscode.Hover(md);
        }

        // impl keyword
        if (word === 'impl') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `impl` — Method Implementation Block\n\n');
            md.appendMarkdown('Groups methods for a struct or enum type.\n\n');
            md.appendMarkdown('```gp\nimpl User {\n    fn Greet(self) -> string {\n        return "Hi, " + self.Name\n    }\n}\n```');
            return new vscode.Hover(md);
        }

        // mut keyword
        if (word === 'mut') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `mut` — Mutable Keyword\n\n');
            md.appendMarkdown('Indicates that a variable or receiver can be mutated.\n\n');
            md.appendMarkdown('```gp\nfn modify(mut self) {\n    self.Count += 1\n}\n```');
            return new vscode.Hover(md);
        }

        // let keyword
        if (word === 'let') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `let` — Variable Declaration\n\n');
            md.appendMarkdown('Declares a variable. Currently compiled to `var` in Go.\n\n');
            md.appendMarkdown('```gp\nlet a = 1\n```');
            return new vscode.Hover(md);
        }

        // const keyword
        if (word === 'const') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `const` — Constant Declaration\n\n');
            md.appendMarkdown('Declares a compile-time constant.\n\n');
            md.appendMarkdown('```gp\nconst PI = 3.14\n```');
            return new vscode.Hover(md);
        }

        // package keyword
        if (word === 'package') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `package` — Package Declaration\n\n');
            md.appendMarkdown('Defines the package name for the current file.\n\n');
            md.appendMarkdown('```gp\npackage main\n```');
            return new vscode.Hover(md);
        }

        // import keyword
        if (word === 'import') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `import` — Import Declaration\n\n');
            md.appendMarkdown('Imports a package to use its exported identifiers.\n\n');
            md.appendMarkdown('```gp\nimport "fmt"\nimport mypkg "github.com/user/pkg"\n```');
            return new vscode.Hover(md);
        }

        // return keyword
        if (word === 'return') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `return` — Return Statement\n\n');
            md.appendMarkdown('Returns from the current function.\n\n');
            return new vscode.Hover(md);
        }

        // Fallback: search for definition to show signature and comment
        if (/^[a-zA-Z_][a-zA-Z0-9_.]*$/.test(word)) {
            const defHover = await this.findDefinitionAndHover(document, position, word);
            if (defHover) {
                return defHover;
            }

            // If not found locally and it looks like a package.symbol, try `go doc`
            if (word.includes('.')) {
                try {
                    const cp = require('child_process');
                    const util = require('util');
                    const execFileAsync = util.promisify(cp.execFile);
                    const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();
                    const { stdout } = await execFileAsync('go', ['doc', word], { cwd });
                    if (stdout) {
                        const md = new vscode.MarkdownString();
                        md.appendCodeblock(stdout.trim(), 'go');
                        return new vscode.Hover(md);
                    }
                } catch {
                    // Ignore errors if go doc fails or is unavailable
                }
            }
        }

        return undefined;
    }

    private async hoverDecorator(document: vscode.TextDocument, position: vscode.Position, name: string, lineText: string): Promise<vscode.Hover | undefined> {
        const doc = DECORATOR_DOCS[name];
        if (!doc) {
            // Unknown/custom decorator
            const md = new vscode.MarkdownString();
            md.appendMarkdown(`### \`@${name}\` — Custom Decorator\n\n`);
            md.appendMarkdown('A user-defined decorator function that wraps the target function.\n\n');
            md.appendMarkdown('Custom decorators take `next` (the original function) and optional arguments, returning a function with the same signature.');
            
            const definitionHover = await this.findDefinitionAndHover(document, position, name);
            if (definitionHover && definitionHover.contents.length > 0) {
                const defContent = definitionHover.contents[0] as vscode.MarkdownString;
                md.appendMarkdown('\n\n**Decorator Definition:**\n\n');
                md.appendMarkdown(defContent.value);
            }
            
            return new vscode.Hover(md);
        }

        const md = new vscode.MarkdownString();
        md.appendMarkdown(`### \`${doc.signature}\`\n\n`);
        md.appendMarkdown(`${doc.description}\n\n`);
        md.appendMarkdown('**Example:**\n');
        md.appendCodeblock(doc.example, 'gp');

        // For @derive, also list available kinds
        if (name === 'derive') {
            md.appendMarkdown('\n**Available derive kinds:**\n\n');
            for (const [kind, desc] of Object.entries(DERIVE_KIND_DOCS)) {
                md.appendMarkdown(`- ${desc}\n`);
            }
        }

        return new vscode.Hover(md);
    }
}
