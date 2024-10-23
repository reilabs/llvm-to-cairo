; ModuleID = '9ox3ykpp0gbrqxqlz7ajwa9w6'
source_filename = "9ox3ykpp0gbrqxqlz7ajwa9w6"
target datalayout = "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128"
target triple = "aarch64-unknown-none"

@alloc_4190527422e5cc48a15bd1cb4f38f425 = private unnamed_addr constant <{ [33 x i8] }> <{ [33 x i8] c"crates/rust-test-input/src/lib.rs" }>, align 1
@alloc_5b4544c775a23c08ca70c48dd7be27fc = private unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @alloc_4190527422e5cc48a15bd1cb4f38f425, [16 x i8] c"!\00\00\00\00\00\00\00\05\00\00\00\05\00\00\00" }>, align 8

; hieratika_rust_test_input::add
; Function Attrs: noredzone nounwind
define dso_local i64 @_ZN19hieratika_rust_test_input3add17h828e50e9267cb510E(i64 %left, i64 %right) unnamed_addr #0 !dbg !5 {
start:
  %right.dbg.spill = alloca [8 x i8], align 8
  %left.dbg.spill = alloca [8 x i8], align 8
  store i64 %left, ptr %left.dbg.spill, align 8
  call void @llvm.dbg.declare(metadata ptr %left.dbg.spill, metadata !12, metadata !DIExpression()), !dbg !15
  store i64 %right, ptr %right.dbg.spill, align 8
  call void @llvm.dbg.declare(metadata ptr %right.dbg.spill, metadata !13, metadata !DIExpression()), !dbg !16
  %0 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %left, i64 %right), !dbg !17
  %_3.0 = extractvalue { i64, i1 } %0, 0, !dbg !17
  %_3.1 = extractvalue { i64, i1 } %0, 1, !dbg !17
  br i1 %_3.1, label %panic, label %bb1, !dbg !17

bb1:                                              ; preds = %start
  ret i64 %_3.0, !dbg !18

panic:                                            ; preds = %start
; call core::panicking::panic_const::panic_const_add_overflow
  call void @_ZN4core9panicking11panic_const24panic_const_add_overflow17he7771b1d81fa091aE(ptr align 8 @alloc_5b4544c775a23c08ca70c48dd7be27fc) #3, !dbg !17
  unreachable, !dbg !17
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare void @llvm.dbg.declare(metadata, metadata, metadata) #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.uadd.with.overflow.i64(i64, i64) #1

; core::panicking::panic_const::panic_const_add_overflow
; Function Attrs: cold noinline noredzone noreturn nounwind
declare dso_local void @_ZN4core9panicking11panic_const24panic_const_add_overflow17he7771b1d81fa091aE(ptr align 8) unnamed_addr #2

attributes #0 = { noredzone nounwind "probe-stack"="inline-asm" "target-cpu"="generic" "target-features"="+v8a,+strict-align,-neon,-fp-armv8" }
attributes #1 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #2 = { cold noinline noredzone noreturn nounwind "probe-stack"="inline-asm" "target-cpu"="generic" "target-features"="+v8a,+strict-align,-neon,-fp-armv8" }
attributes #3 = { noreturn nounwind }

!llvm.ident = !{!0}
!llvm.dbg.cu = !{!1}
!llvm.module.flags = !{!3, !4}

!0 = !{!"rustc version 1.81.0 (eeb90cda1 2024-09-04)"}
!1 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !2, producer: "clang LLVM (rustc version 1.81.0 (eeb90cda1 2024-09-04))", isOptimized: false, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!2 = !DIFile(filename: "crates/rust-test-input/src/lib.rs/@/9ox3ykpp0gbrqxqlz7ajwa9w6", directory: "/Users/starfire/Development/reilabs/starkware/hieratika")
!3 = !{i32 2, !"Dwarf Version", i32 4}
!4 = !{i32 2, !"Debug Info Version", i32 3}
!5 = distinct !DISubprogram(name: "add", linkageName: "_ZN19hieratika_rust_test_input3add17h828e50e9267cb510E", scope: !7, file: !6, line: 4, type: !8, scopeLine: 4, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition, unit: !1, templateParams: !14, retainedNodes: !11)
!6 = !DIFile(filename: "crates/rust-test-input/src/lib.rs", directory: "/Users/starfire/Development/reilabs/starkware/hieratika", checksumkind: CSK_MD5, checksum: "178b5b568f49bd1e17834a7529756af1")
!7 = !DINamespace(name: "hieratika_rust_test_input", scope: null)
!8 = !DISubroutineType(types: !9)
!9 = !{!10, !10, !10}
!10 = !DIBasicType(name: "u64", size: 64, encoding: DW_ATE_unsigned)
!11 = !{!12, !13}
!12 = !DILocalVariable(name: "left", arg: 1, scope: !5, file: !6, line: 4, type: !10)
!13 = !DILocalVariable(name: "right", arg: 2, scope: !5, file: !6, line: 4, type: !10)
!14 = !{}
!15 = !DILocation(line: 4, column: 12, scope: !5)
!16 = !DILocation(line: 4, column: 23, scope: !5)
!17 = !DILocation(line: 5, column: 5, scope: !5)
!18 = !DILocation(line: 6, column: 2, scope: !5)
