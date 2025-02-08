# Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
# Unauthorized copying of this file, via any medium is strictly prohibited.
# Proprietary and confidential.

load("@ainc//build_defs:rust_library.bzl", "rust_library")

rust_library(
    name = "shrubbery",
    srcs = glob(["src/**/*.rs"]),
    features = [],
    deps = [
        # keep sorted
        "third_party//rust:ahash",
        "third_party//rust:derive_more",
        "third_party//rust:graphviz-rust",
        "third_party//rust:log",
        "third_party//rust:regex",
        "third_party//rust:thiserror",
    ],
    test_deps = [],
    examples = [
        # keep sorted
        "animation",
    ],
    tests = [
        # keep sorted
        "control_tree",
    ],
    visibility = ["PUBLIC"],
)
