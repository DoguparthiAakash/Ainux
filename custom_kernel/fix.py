import sys
content = open("build.rs").read()
content = content.replace("src/c/", "c_src/")
content = content.replace("src/c\"", "c_src\"")
content = content.replace("src/asm/", "arch/x86_64/asm/")
content = content.replace("src/bsd_", "c_src/bsd_")
# fix include path
content = content.replace("format!(\"-I{}/include\", c_dir_wsl),", "format!(\"-I{}/include\", wsl_manifest),")

# Wait, the cc::Build section also has .include("src/c/include") which became .include("c_src/include") but should be .include("include").
content = content.replace(".include(\"c_src/include\")", ".include(\"include\")")

open("build.rs", "w").write(content)
