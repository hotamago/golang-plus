use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_pattern(&mut self, raw: &str, span: Span) -> Pattern {
        let text = raw.trim();
        if text == "_" {
            return Pattern::Wildcard { span };
        }

        if let Some((left, right)) = text.split_once("::") {
            let enum_name = left.trim().to_string();
            let (variant, bindings) = parse_pattern_variant(right.trim());
            return Pattern::TypedVariant {
                enum_name,
                variant,
                bindings,
                span,
            };
        }

        let (variant, bindings) = parse_pattern_variant(text);
        Pattern::Variant {
            variant,
            bindings,
            span,
        }
    }
}

fn parse_pattern_variant(input: &str) -> (String, Vec<String>) {
    let text = input.trim();
    if let Some(lp) = text.find('(')
        && text.ends_with(')')
    {
        let name = text[..lp].trim().to_string();
        let inside = &text[lp + 1..text.len() - 1];
        let bindings = inside
            .split(',')
            .map(str::trim)
            .filter(|it| !it.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        return (name, bindings);
    }
    (text.to_string(), Vec::new())
}
