use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use dialoguer::console::style;

struct Resource {
    repo: &'static str,
    description: &'static str,
    category: Category,
}

#[derive(Clone, Copy)]
enum Category {
    Analysis,
    Tutorials,
    Compilers,
    Decompilers,
    MLIR,
    Toolchains,
    Security,
    RustTools,
    Bindings,
    Optimization,
    Misc,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Category::Analysis => "Analysis & Verification",
            Category::Tutorials => "Tutorials & Learning",
            Category::Compilers => "Compilers & Frontends",
            Category::Decompilers => "Decompilers & Lifters",
            Category::MLIR => "MLIR",
            Category::Toolchains => "Toolchains & Build",
            Category::Security => "Security & Obfuscation",
            Category::RustTools => "Rust LLVM Tools",
            Category::Bindings => "Language Bindings",
            Category::Optimization => "Optimization & Transforms",
            Category::Misc => "Miscellaneous",
        }
    }

    fn emoji(self) -> &'static str {
        match self {
            Category::Analysis => "🔬",
            Category::Tutorials => "📚",
            Category::Compilers => "⚙️",
            Category::Decompilers => "🔄",
            Category::MLIR => "🧩",
            Category::Toolchains => "🔧",
            Category::Security => "🔒",
            Category::RustTools => "🦀",
            Category::Bindings => "🔗",
            Category::Optimization => "⚡",
            Category::Misc => "📦",
        }
    }
}

use Category::*;

const RESOURCES: &[Resource] = &[
    // ── Analysis & Verification ──
    Resource { repo: "AliveToolkit/alive2", description: "Automatic verification of LLVM optimizations", category: Analysis },
    Resource { repo: "nunoplopes/alive", description: "Automatic LLVM InstCombine verifier", category: Analysis },
    Resource { repo: "microsoft/AliveInLean", description: "Formally verified implementation of Alive in Lean", category: Analysis },
    Resource { repo: "secure-software-engineering/phasar", description: "LLVM-based static analysis framework", category: Analysis },
    Resource { repo: "SVF-tools/SVF", description: "Static Value-Flow Analysis Framework for source code", category: Analysis },
    Resource { repo: "mchalupa/dg", description: "Dependence graphs and program slicing of LLVM bitcode", category: Analysis },
    Resource { repo: "seahorn/seahorn", description: "SeaHorn Verification Framework", category: Analysis },
    Resource { repo: "seahorn/clam", description: "Static analyzer for LLVM bitcode based on abstract interpretation", category: Analysis },
    Resource { repo: "seahorn/sea-dsa", description: "Context, field, and array-sensitive heap analysis for LLVM", category: Analysis },
    Resource { repo: "grievejia/andersen", description: "Andersen's pointer analysis re-implementation for LLVM", category: Analysis },
    Resource { repo: "GaloisInc/cclyzerpp", description: "Precise and scalable pointer analysis for LLVM code", category: Analysis },
    Resource { repo: "plast-lab/cclyzer", description: "Analyzing LLVM bitcode using Datalog", category: Analysis },
    Resource { repo: "seclab-ucr/SUTURE", description: "Precise static points-to/taint analysis based on LLVM IR", category: Analysis },
    Resource { repo: "compor/Pedigree", description: "LLVM dependence graphs", category: Analysis },
    Resource { repo: "trailofbits/polytracker", description: "LLVM instrumentation for taint tracking and dataflow analysis", category: Analysis },
    Resource { repo: "harvard-acc/LLVM-Tracer", description: "LLVM pass to profile dynamic IR instructions and runtime values", category: Analysis },
    Resource { repo: "smackers/smack", description: "SMACK Software Verifier and Verification Toolchain", category: Analysis },
    Resource { repo: "dtcxzyw/llvm-ub-aware-interpreter", description: "UB-aware interpreter for debugging LLVM", category: Analysis },
    Resource { repo: "Fraunhofer-AISEC/cpg", description: "Code Property Graph extraction from C/C++, Java, Go, Python & LLVM-IR", category: Analysis },
    Resource { repo: "ShiftLeftSecurity/llvm2cpg", description: "LLVM meets Code Property Graphs", category: Analysis },
    Resource { repo: "rcorcs/llvm-heat-printer", description: "LLVM profiling visualization", category: Analysis },
    Resource { repo: "IITH-Compilers/IR2Vec", description: "LLVM IR based scalable program embeddings", category: Analysis },
    Resource { repo: "sfu-arch/llvm-epp", description: "Efficient Path Profiling using LLVM", category: Analysis },
    Resource { repo: "fundamental/stoat", description: "Static LLVM Object file Analysis Tool", category: Analysis },
    Resource { repo: "kframework/llvm-semantics", description: "Formal semantics of LLVM IR in K", category: Analysis },
    Resource { repo: "termite-analyser/llvm2smt", description: "OCaml library to transform LLVM CFG into SMT formula", category: Analysis },

    // ── Tutorials & Learning ──
    Resource { repo: "banach-space/llvm-tutor", description: "Collection of out-of-tree LLVM passes for teaching and learning", category: Tutorials },
    Resource { repo: "mikeroyal/LLVM-Guide", description: "Comprehensive guide to LLVM compiler infrastructure", category: Tutorials },
    Resource { repo: "learn-llvm/awesome-llvm", description: "Curated list of awesome LLVM related resources", category: Tutorials },
    Resource { repo: "Evian-Zhang/llvm-ir-tutorial", description: "LLVM IR introductory tutorial (Chinese)", category: Tutorials },
    Resource { repo: "lijiansong/clang-llvm-tutorial", description: "Clang & LLVM examples: AST interpreter, pointer analysis, backend", category: Tutorials },
    Resource { repo: "sampsyo/llvm-pass-skeleton", description: "Example LLVM pass skeleton", category: Tutorials },
    Resource { repo: "urlyy/llvm-new-pass-tutor", description: "Guide to writing LLVM passes for beginners", category: Tutorials },
    Resource { repo: "nael8r/How-To-Write-An-LLVM-Register-Allocator", description: "Tutorial for writing an LLVM register allocator", category: Tutorials },
    Resource { repo: "sdiehl/kaleidoscope", description: "Haskell LLVM JIT Compiler Tutorial", category: Tutorials },
    Resource { repo: "adamrk/llvm-ocaml-tutorial", description: "The LLVM tutorial using OCaml", category: Tutorials },
    Resource { repo: "DmitrySoshnikov/eva-llvm-source", description: "Source code for \"Programming Language with LLVM\" class", category: Tutorials },
    Resource { repo: "tuoxie007/play_with_llvm", description: "A book about LLVM & Clang (Chinese)", category: Tutorials },
    Resource { repo: "bigconvience/llvm-ir-in-action", description: "LLVM IR in action examples", category: Tutorials },
    Resource { repo: "nsumner/llvm-demo", description: "Using LLVM to gather static or dynamic facts about a program", category: Tutorials },
    Resource { repo: "UofT-EcoSystem/CSCD70", description: "CSCD70 Compiler Optimization course (Univ. of Toronto)", category: Tutorials },
    Resource { repo: "adamtiger/tinyGPUlang", description: "Tutorial on building a GPU compiler backend in LLVM", category: Tutorials },
    Resource { repo: "LLVMParty/LLVMCMakeTemplate", description: "CMake scripts to easily link LLVM into your project", category: Tutorials },
    Resource { repo: "pfalcon/graph-llvm-ir", description: "Visualization of LLVM IR", category: Tutorials },

    // ── Compilers & Frontends ──
    Resource { repo: "flang-compiler/flang", description: "Fortran language front-end for integration with LLVM", category: Compilers },
    Resource { repo: "luc-tielen/eclair-lang", description: "Minimal, fast Datalog implementation compiling to LLVM IR", category: Compilers },
    Resource { repo: "grin-compiler/grin", description: "Compiler back-end for lazy/strict functional languages", category: Compilers },
    Resource { repo: "matthewbdwyer/tipc", description: "A compiler from TIP to LLVM bitcode", category: Compilers },
    Resource { repo: "anhnguyen1618/Tiger-ocaml-llvm-compiler", description: "Minimal compiler in OCaml compiling Tiger to LLVM IR", category: Compilers },
    Resource { repo: "colinbenner/ocaml-llvm", description: "LLVM-based backend for OCaml", category: Compilers },
    Resource { repo: "yrnkrn/zapcc", description: "Caching C++ compiler based on clang", category: Compilers },
    Resource { repo: "simit-lang/simit", description: "A language for computing on sparse systems", category: Compilers },
    Resource { repo: "leanprover/lean-llvm", description: "Custom-built LLVM toolchain for Lean 4", category: Compilers },
    Resource { repo: "bytedance/byteir", description: "Model compilation solution for various hardware", category: Compilers },
    Resource { repo: "moonbitlang/MoonLLVM", description: "A tiny, friendly companion to LLVM", category: Compilers },
    Resource { repo: "sillycross/PochiVM", description: "Lightweight framework for easy and efficient code generation", category: Compilers },
    Resource { repo: "paradigmxyz/revmc", description: "JIT and AOT compiler for the Ethereum Virtual Machine", category: Compilers },
    Resource { repo: "NilFoundation/zkLLVM", description: "Zero-Knowledge Proof Systems Circuit Compiler", category: Compilers },
    Resource { repo: "bluescarni/heyoka", description: "C++ ODE integration via Taylor's method and LLVM", category: Compilers },
    Resource { repo: "checkedc/checkedc-clang", description: "Clang modified to support Checked C (type-safe C extension)", category: Compilers },

    // ── Decompilers & Lifters ──
    Resource { repo: "avast/retdec", description: "Retargetable machine-code decompiler based on LLVM", category: Decompilers },
    Resource { repo: "lifting-bits/remill", description: "Library for lifting machine code to LLVM bitcode", category: Decompilers },
    Resource { repo: "lifting-bits/rellic", description: "Produces goto-free C output from LLVM bitcode", category: Decompilers },
    Resource { repo: "microsoft/llvm-mctoll", description: "Machine code to LLVM IR translator", category: Decompilers },
    Resource { repo: "trailofbits/circuitous", description: "Binary → LLVM → circuits", category: Decompilers },
    Resource { repo: "NaC-L/Mergen", description: "Deobfuscation via optimization using LLVM IR and assembly", category: Decompilers },
    Resource { repo: "RPISEC/llvm-deobfuscator", description: "LLVM-based deobfuscator", category: Decompilers },

    // ── MLIR ──
    Resource { repo: "llvm/circt", description: "Circuit IR Compilers and Tools", category: MLIR },
    Resource { repo: "mlir-rs/melior", description: "Rustic MLIR bindings in Rust", category: MLIR },
    Resource { repo: "Lewuathe/mlir-hello", description: "MLIR sample dialect", category: MLIR },
    Resource { repo: "jmgorius/mlir-standalone-template", description: "Out-of-tree MLIR dialect template", category: MLIR },
    Resource { repo: "libxsmm/tpp-mlir", description: "TPP experimentation on MLIR for linear algebra", category: MLIR },

    // ── Toolchains & Build ──
    Resource { repo: "ARM-software/LLVM-embedded-toolchain-for-Arm", description: "LLVM toolchain for Arm and AArch64 embedded targets", category: Toolchains },
    Resource { repo: "arm/arm-toolchain", description: "LLVM toolchain for Arm/AArch64 embedded & native Linux dev", category: Toolchains },
    Resource { repo: "rsms/llvmbox", description: "Self-contained, fully static LLVM tools & libs", category: Toolchains },
    Resource { repo: "osquery/osquery-toolchain", description: "LLVM-based toolchain designed to build a portable osquery", category: Toolchains },
    Resource { repo: "llvmenv/llvmenv", description: "Manage multiple LLVM/Clang builds", category: Toolchains },
    Resource { repo: "cerisier/toolchains_llvm_bootstrapped", description: "Zero-sysroot, hermetic C/C++ cross-compilation for Bazel", category: Toolchains },
    Resource { repo: "bazelembedded/rules_cc_toolchain", description: "Hermetic toolchain for Bazel", category: Toolchains },
    Resource { repo: "ROCm/aomp", description: "Open source Clang/LLVM compiler with OpenMP on Radeon GPUs", category: Toolchains },
    Resource { repo: "CTSRD-CHERI/llvm-project", description: "Fork of LLVM adding CHERI support", category: Toolchains },
    Resource { repo: "ClickHouse/llvm", description: "Stripped LLVM for runtime code generation in ClickHouse", category: Toolchains },
    Resource { repo: "t-crest/patmos-llvm", description: "LLVM compiler port for the time-predictable Patmos processor", category: Toolchains },
    Resource { repo: "light-tech/LLVM-On-iOS", description: "Build LLVM/Clang for iOS with example C++ interpreter", category: Toolchains },
    Resource { repo: "opencollab/llvm-toolchain-integration-test-suite", description: "Integration tests for the LLVM toolchain", category: Toolchains },
    Resource { repo: "llvm/llvm-test-suite", description: "Official LLVM test suite", category: Toolchains },
    Resource { repo: "remotemcu/adin-llvm", description: "Specialized LLVM compiler with ADIN code transformer pass", category: Toolchains },
    Resource { repo: "apc-llc/nvcc-llvm-ir", description: "Manipulating LLVM IR from CUDA sources on the fly", category: Toolchains },

    // ── Security & Obfuscation ──
    Resource { repo: "eshard/obfuscator-llvm", description: "LLVM obfuscation plugin pass", category: Security },
    Resource { repo: "lich4/awesome-ollvm", description: "Awesome Obfuscator-LLVMs and IDA deobfuscation plugins", category: Security },
    Resource { repo: "gmh5225/awesome-llvm-security", description: "Curated list of LLVM security resources", category: Security },
    Resource { repo: "hanswinderix/sllvm", description: "Security Enhanced LLVM", category: Security },
    Resource { repo: "SheLLVM/SheLLVM", description: "LLVM passes to write shellcode in regular C", category: Security },
    Resource { repo: "ant4g0nist/ManuFuzzer", description: "Binary code-coverage fuzzer for macOS based on libFuzzer/LLVM", category: Security },
    Resource { repo: "trailofbits/ebpfault", description: "BPF-based syscall fault injector", category: Security },

    // ── Rust LLVM Tools ──
    Resource { repo: "TheDan64/inkwell", description: "Safe LLVM wrapper for Rust (INKWELL)", category: RustTools },
    Resource { repo: "woodruffw/mollusc", description: "Pure-Rust libraries for parsing and analyzing LLVM", category: RustTools },
    Resource { repo: "cdisselkoen/llvm-ir", description: "LLVM IR in natural Rust data structures", category: RustTools },
    Resource { repo: "cdisselkoen/llvm-ir-analysis", description: "Analysis tools for LLVM IR in Rust", category: RustTools },
    Resource { repo: "jamesmth/llvm-plugin-rs", description: "Out-of-tree LLVM passes in Rust", category: RustTools },
    Resource { repo: "taiki-e/cargo-llvm-cov", description: "Cargo subcommand for LLVM source-based code coverage", category: RustTools },
    Resource { repo: "pacak/cargo-show-asm", description: "Cargo subcommand showing assembly, LLVM-IR and MIR", category: RustTools },
    Resource { repo: "gnzlbg/cargo-asm", description: "Cargo subcommand showing assembly or LLVM-IR", category: RustTools },
    Resource { repo: "dtolnay/cargo-llvm-lines", description: "Count lines of LLVM IR per generic function", category: RustTools },
    Resource { repo: "Kobzol/cargo-remark", description: "Cargo subcommand for viewing LLVM optimization remarks", category: RustTools },
    Resource { repo: "rust-embedded/cargo-binutils", description: "Cargo subcommands to invoke LLVM tools from Rust toolchain", category: RustTools },
    Resource { repo: "xd009642/llvm-profparser", description: "Pure Rust parser for LLVM instrumentation profile data", category: RustTools },
    Resource { repo: "rustfoundation/painter", description: "Ecosystem-wide call graphs and LLVM-IR analysis", category: RustTools },

    // ── Language Bindings ──
    Resource { repo: "ruby-llvm/ruby-llvm", description: "Ruby bindings for LLVM", category: Bindings },
    Resource { repo: "llvm-hs/llvm-hs-pretty", description: "Pretty printer for LLVM AST to textual IR (Haskell)", category: Bindings },
    Resource { repo: "AccelerateHS/accelerate", description: "Embedded language for high-performance array computations", category: Bindings },
    Resource { repo: "llir/grammar", description: "EBNF grammar of LLVM IR assembly", category: Bindings },
    Resource { repo: "KhronosGroup/SPIRV-LLVM-Translator", description: "Bi-directional translation between SPIR-V and LLVM IR", category: Bindings },
    Resource { repo: "qir-alliance/pyqir", description: "APIs for generating and parsing Quantum IR (LLVM-based)", category: Bindings },
    Resource { repo: "qir-alliance/qir-spec", description: "QIR spec for quantum programs within LLVM IR", category: Bindings },

    // ── Optimization & Transforms ──
    Resource { repo: "EnzymeAD/Enzyme", description: "High-performance automatic differentiation of LLVM and MLIR", category: Optimization },
    Resource { repo: "google/souper", description: "A superoptimizer for LLVM IR", category: Optimization },
    Resource { repo: "google/llvm-propeller", description: "Profile-guided optimizing large-scale LLVM-based relinker", category: Optimization },
    Resource { repo: "cdl-saarland/rv", description: "RV: A Unified Region Vectorizer for LLVM", category: Optimization },
    Resource { repo: "michalpaszkowski/LLVM-Canon", description: "Transforms LLVM modules into canonical form", category: Optimization },
    Resource { repo: "travitch/whole-program-llvm", description: "Wrapper to build whole-program LLVM bitcode files", category: Optimization },
    Resource { repo: "jotaviobiondo/llvm-register-allocator", description: "Graph coloring register allocator for LLVM", category: Optimization },

    // ── Miscellaneous ──
    Resource { repo: "emproof-com/nyxstone", description: "Assembly/disassembly library based on LLVM", category: Misc },
    Resource { repo: "tudasc/TypeART", description: "LLVM type and memory allocation tracking sanitizer", category: Misc },
    Resource { repo: "tudasc/MetaCG", description: "Annotated whole-program call-graph tool for Clang/LLVM", category: Misc },
    Resource { repo: "mull-project/libirm", description: "Low-level IR mutations for LLVM bitcode", category: Misc },
    Resource { repo: "tum-ei-eda/seal5", description: "Semi-automated LLVM support for RISC-V extensions", category: Misc },
    Resource { repo: "Guardsquare/LibEBC", description: "C++ library for extracting embedded bitcode", category: Misc },
    Resource { repo: "eunomia-bpf/bpftime", description: "Userspace eBPF runtime for observability and networking", category: Misc },
    Resource { repo: "eunomia-bpf/llvmbpf", description: "Userspace/GPU eBPF VM with LLVM JIT/AOT compiler", category: Misc },
    Resource { repo: "vaivaswatha/debugir", description: "DebugIR: Debugging LLVM-IR files", category: Misc },
    Resource { repo: "cisco-open/llvm-crash-analyzer", description: "LLVM crash analysis", category: Misc },
    Resource { repo: "s3cur3/llvm-data-structure-benchmarks", description: "Benchmark for cache-efficient data structures", category: Misc },
];

pub fn show_resources() {
    let all_categories: &[Category] = &[
        Analysis, Tutorials, Compilers, Decompilers, MLIR,
        Toolchains, Security, RustTools, Bindings, Optimization, Misc,
    ];

    let cat_items: Vec<String> = all_categories.iter()
        .map(|c| {
            let count = RESOURCES.iter().filter(|r| r.category as u8 == *c as u8).count();
            format!("{} {} ({})", c.emoji(), c.label(), count)
        })
        .collect();

    loop {
        println!("\n{}\n", style("Interesting LLVM Resources").yellow().bold());

        let mut items = cat_items.clone();
        items.push(format!("{} Browse all ({} repos)", "🌐", RESOURCES.len()));
        items.push("← Back".to_string());

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Pick a category (type to search)")
            .items(&items)
            .default(0)
            .interact_opt()
            .expect("failed to render menu");

        let Some(idx) = selection else { return; };

        if idx == items.len() - 1 {
            return;
        }

        if idx == items.len() - 2 {
            browse_resources(RESOURCES);
        } else {
            let cat = all_categories[idx];
            let filtered: Vec<&Resource> = RESOURCES.iter()
                .filter(|r| r.category as u8 == cat as u8)
                .collect();
            println!("\n  {} {}\n", cat.emoji(), style(cat.label()).bold());
            browse_resources_refs(&filtered);
        }
    }
}

fn browse_resources(resources: &[Resource]) {
    let refs: Vec<&Resource> = resources.iter().collect();
    browse_resources_refs(&refs);
}

fn browse_resources_refs(resources: &[&Resource]) {
    let items: Vec<String> = resources.iter()
        .map(|r| format!("{} {} — {}", r.category.emoji(), r.repo, style(r.description).dim()))
        .collect();

    let mut display_items = items.clone();
    display_items.push("← Back".to_string());

    loop {
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a repo to open (type to search)")
            .items(&display_items)
            .default(0)
            .interact_opt()
            .expect("failed to render menu");

        let Some(idx) = selection else { return; };

        if idx == display_items.len() - 1 {
            return;
        }

        let resource = resources[idx];
        let url = format!("https://github.com/{}", resource.repo);
        println!("\n  {} {}", style("→").green().bold(), style(&url).cyan().underlined());
        println!("    {}\n", style(resource.description).dim());

        // Try to open in browser
        let _ = std::process::Command::new("open")
            .arg(&url)
            .status();
    }
}
