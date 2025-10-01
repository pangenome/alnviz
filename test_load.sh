#!/bin/bash
# Automated test for file loading

set -e

TEST_FILE="/home/erik/human/h.51.1aln"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ALNview File Loading Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "File: $TEST_FILE"
echo "Size: $(du -h "$TEST_FILE" | cut -f1)"
echo ""

# Build first
echo "📦 Building..."
cargo build --quiet 2>&1 | tail -5 || true
echo ""

# Test 1: Try loading with GDB to get stack trace
echo "🔍 Test 1: GDB Stack Trace"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

timeout 5 gdb -batch -ex "run" -ex "bt" -ex "quit" \
    --args target/debug/alnview 2>&1 | grep -A 20 "Program received" || echo "No crash in GDB (needs manual load)"

echo ""

# Test 2: Try with Valgrind
echo "🔍 Test 2: Valgrind"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

timeout 10 valgrind --leak-check=no target/debug/alnview 2>&1 | grep -E "(SIGSEGV|FPE|Invalid)" | head -10 || echo "Valgrind: No issues detected (or needs manual file load)"

echo ""

# Test 3: C-only test (bypass Rust GUI)
echo "🔍 Test 3: Direct C Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cat > /tmp/test_c_load.c << 'CEOF'
#include <stdio.h>
#include <stdlib.h>

extern void* createPlot(const char* path, int lCut, int iCut, int sCut, void* model);
extern void Free_DotPlot(void* plot);

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <file.1aln>\n", argv[0]);
        return 1;
    }

    printf("📞 Calling createPlot('%s')...\n", argv[1]);
    fflush(stdout);

    void* plot = createPlot(argv[1], 0, 0, 0, NULL);

    if (plot) {
        printf("✅ SUCCESS: Plot created at %p\n", plot);
        Free_DotPlot(plot);
        return 0;
    } else {
        printf("❌ FAILED: createPlot returned NULL\n");
        return 1;
    }
}
CEOF

# Compile C test linking against our library
gcc -o /tmp/test_c_load /tmp/test_c_load.c \
    -L./target/debug -lalnview_c \
    -Wl,-rpath,./target/debug \
    -lz 2>/dev/null || {
    echo "⚠️  Could not compile C test (library not found)"
}

if [ -f /tmp/test_c_load ]; then
    timeout 5 /tmp/test_c_load "$TEST_FILE" 2>&1 || {
        EXITCODE=$?
        case $EXITCODE in
            124) echo "⏱️  Timeout - createPlot() hung" ;;
            136) echo "💥 SIGFPE - Floating point exception (divide by zero?)" ;;
            139) echo "💥 SIGSEGV - Segmentation fault" ;;
            *) echo "💥 Exit code: $EXITCODE" ;;
        esac
    }
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Test Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 To debug manually:"
echo "   gdb target/debug/alnview"
echo "   > run"
echo "   > (click Open, select file)"
echo "   > bt  (when it crashes)"
echo ""
echo "🔧 To fix:"
echo "   1. Find divide-by-zero in C code"
echo "   2. Add null checks before division"
echo "   3. Or port to Rust (Week 3-4)"
EOF

chmod +x test_load.sh
./test_load.sh
