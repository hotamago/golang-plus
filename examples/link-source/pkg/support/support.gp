package support

import "example.com/goplus-link-source/internal/label"

fn FromNestedPackages() -> string {
    return label.Prefix() + " support.FromNestedPackages()"
}
