# Injected into the whisper.cpp CMake build for release installers.
# Without this, ggml compiles with -march=native for the CI machine, and the
# app crashes with "illegal instruction" on CPUs lacking the runner's
# extensions. OFF selects a portable baseline (x86_64: AVX2/FMA/F16C; ARM: NEON).
set(GGML_NATIVE OFF CACHE BOOL "Portable DictaFox release build" FORCE)
