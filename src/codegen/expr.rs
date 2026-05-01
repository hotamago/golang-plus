use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn transform_expr(&self, text: &str) -> String {
        let mut out = text.to_string();
        for enum_decl in self.model.enums.values() {
            for variant in &enum_decl.variants {
                if enum_decl.is_tagged() {
                    let re_generic = Regex::new(&format!(
                        r"\b{}\s*<([^>]*)>\s*::\s*{}\s*\(",
                        regex::escape(&enum_decl.name),
                        regex::escape(&variant.name)
                    ))
                    .expect("valid regex");
                    out = re_generic
                        .replace_all(&out, format!("{}{}[$1](", enum_decl.name, variant.name))
                        .to_string();

                    let re_plain = Regex::new(&format!(
                        r"\b{}\s*::\s*{}\s*\(",
                        regex::escape(&enum_decl.name),
                        regex::escape(&variant.name)
                    ))
                    .expect("valid regex");
                    out = re_plain
                        .replace_all(&out, format!("{}{}(", enum_decl.name, variant.name))
                        .to_string();
                } else {
                    let re = Regex::new(&format!(
                        r"\b{}\s*::\s*{}\b",
                        regex::escape(&enum_decl.name),
                        regex::escape(&variant.name)
                    ))
                    .expect("valid regex");
                    out = re
                        .replace_all(&out, format!("{}{}", enum_decl.name, variant.name))
                        .to_string();
                }
            }
        }
        out = rewrite_fn_syntax(&out);
        out
    }

    pub(super) fn next_tmp(&mut self, prefix: &str) -> String {
        let id = self.tmp_counter;
        self.tmp_counter += 1;
        format!("{}{}", prefix, id)
    }
}
