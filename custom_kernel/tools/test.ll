; NUX Native LLVM IR Compiler
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-none-elf"

@vm_stack = global [1024 x i64] zeroinitializer
@vm_stack_ptr = global i64 0

define i64 @_start() {
entry:
  %sp_val_0 = load i64, ptr @vm_stack_ptr
  %sp_ptr_0 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_0
  store i64 72, ptr %sp_ptr_0
  %sp_next_0 = add i64 %sp_val_0, 1
  store i64 %sp_next_0, ptr @vm_stack_ptr
  %sp_curr_char_1 = load i64, ptr @vm_stack_ptr
  %sp_prev_char_1 = sub i64 %sp_curr_char_1, 1
  store i64 %sp_prev_char_1, ptr @vm_stack_ptr
  %sp_ptr_char_1 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_char_1
  %char_1 = load i64, ptr %sp_ptr_char_1
  %buf_1 = alloca i8, align 1
  %tr_1 = trunc i64 %char_1 to i8
  store i8 %tr_1, ptr %buf_1
  %buf_i64_1 = ptrtoint ptr %buf_1 to i64
  call void asm sideeffect "syscall", "{rax},{rdi},{rsi},{rdx},~{rcx},~{r11},~{memory}"(i64 1, i64 1, i64 %buf_i64_1, i64 1)
  %sp_val_2 = load i64, ptr @vm_stack_ptr
  %sp_ptr_2 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_2
  store i64 101, ptr %sp_ptr_2
  %sp_next_2 = add i64 %sp_val_2, 1
  store i64 %sp_next_2, ptr @vm_stack_ptr
  %sp_curr_char_3 = load i64, ptr @vm_stack_ptr
  %sp_prev_char_3 = sub i64 %sp_curr_char_3, 1
  store i64 %sp_prev_char_3, ptr @vm_stack_ptr
  %sp_ptr_char_3 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_char_3
  %char_3 = load i64, ptr %sp_ptr_char_3
  %buf_3 = alloca i8, align 1
  %tr_3 = trunc i64 %char_3 to i8
  store i8 %tr_3, ptr %buf_3
  %buf_i64_3 = ptrtoint ptr %buf_3 to i64
  call void asm sideeffect "syscall", "{rax},{rdi},{rsi},{rdx},~{rcx},~{r11},~{memory}"(i64 1, i64 1, i64 %buf_i64_3, i64 1)
  %sp_val_4 = load i64, ptr @vm_stack_ptr
  %sp_ptr_4 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_4
  store i64 108, ptr %sp_ptr_4
  %sp_next_4 = add i64 %sp_val_4, 1
  store i64 %sp_next_4, ptr @vm_stack_ptr
  %sp_curr_char_5 = load i64, ptr @vm_stack_ptr
  %sp_prev_char_5 = sub i64 %sp_curr_char_5, 1
  store i64 %sp_prev_char_5, ptr @vm_stack_ptr
  %sp_ptr_char_5 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_char_5
  %char_5 = load i64, ptr %sp_ptr_char_5
  %buf_5 = alloca i8, align 1
  %tr_5 = trunc i64 %char_5 to i8
  store i8 %tr_5, ptr %buf_5
  %buf_i64_5 = ptrtoint ptr %buf_5 to i64
  call void asm sideeffect "syscall", "{rax},{rdi},{rsi},{rdx},~{rcx},~{r11},~{memory}"(i64 1, i64 1, i64 %buf_i64_5, i64 1)
  %sp_val_6 = load i64, ptr @vm_stack_ptr
  %sp_ptr_6 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_6
  store i64 108, ptr %sp_ptr_6
  %sp_next_6 = add i64 %sp_val_6, 1
  store i64 %sp_next_6, ptr @vm_stack_ptr
  %sp_curr_char_7 = load i64, ptr @vm_stack_ptr
  %sp_prev_char_7 = sub i64 %sp_curr_char_7, 1
  store i64 %sp_prev_char_7, ptr @vm_stack_ptr
  %sp_ptr_char_7 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_char_7
  %char_7 = load i64, ptr %sp_ptr_char_7
  %buf_7 = alloca i8, align 1
  %tr_7 = trunc i64 %char_7 to i8
  store i8 %tr_7, ptr %buf_7
  %buf_i64_7 = ptrtoint ptr %buf_7 to i64
  call void asm sideeffect "syscall", "{rax},{rdi},{rsi},{rdx},~{rcx},~{r11},~{memory}"(i64 1, i64 1, i64 %buf_i64_7, i64 1)
  %sp_val_8 = load i64, ptr @vm_stack_ptr
  %sp_ptr_8 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_8
  store i64 111, ptr %sp_ptr_8
  %sp_next_8 = add i64 %sp_val_8, 1
  store i64 %sp_next_8, ptr @vm_stack_ptr
  %sp_curr_char_9 = load i64, ptr @vm_stack_ptr
  %sp_prev_char_9 = sub i64 %sp_curr_char_9, 1
  store i64 %sp_prev_char_9, ptr @vm_stack_ptr
  %sp_ptr_char_9 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_char_9
  %char_9 = load i64, ptr %sp_ptr_char_9
  %buf_9 = alloca i8, align 1
  %tr_9 = trunc i64 %char_9 to i8
  store i8 %tr_9, ptr %buf_9
  %buf_i64_9 = ptrtoint ptr %buf_9 to i64
  call void asm sideeffect "syscall", "{rax},{rdi},{rsi},{rdx},~{rcx},~{r11},~{memory}"(i64 1, i64 1, i64 %buf_i64_9, i64 1)
  %sp_val_10 = load i64, ptr @vm_stack_ptr
  %sp_ptr_10 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_10
  store i64 10, ptr %sp_ptr_10
  %sp_next_10 = add i64 %sp_val_10, 1
  store i64 %sp_next_10, ptr @vm_stack_ptr
  %sp_curr_char_11 = load i64, ptr @vm_stack_ptr
  %sp_prev_char_11 = sub i64 %sp_curr_char_11, 1
  store i64 %sp_prev_char_11, ptr @vm_stack_ptr
  %sp_ptr_char_11 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_char_11
  %char_11 = load i64, ptr %sp_ptr_char_11
  %buf_11 = alloca i8, align 1
  %tr_11 = trunc i64 %char_11 to i8
  store i8 %tr_11, ptr %buf_11
  %buf_i64_11 = ptrtoint ptr %buf_11 to i64
  call void asm sideeffect "syscall", "{rax},{rdi},{rsi},{rdx},~{rcx},~{r11},~{memory}"(i64 1, i64 1, i64 %buf_i64_11, i64 1)
  %sp_val_12 = load i64, ptr @vm_stack_ptr
  %sp_ptr_12 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_12
  store i64 0, ptr %sp_ptr_12
  %sp_next_12 = add i64 %sp_val_12, 1
  store i64 %sp_next_12, ptr @vm_stack_ptr
  %sp_curr_exit_code_13 = load i64, ptr @vm_stack_ptr
  %sp_prev_exit_code_13 = sub i64 %sp_curr_exit_code_13, 1
  store i64 %sp_prev_exit_code_13, ptr @vm_stack_ptr
  %sp_ptr_exit_code_13 = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_exit_code_13
  %exit_code_13 = load i64, ptr %sp_ptr_exit_code_13
  call void asm sideeffect "syscall", "{rax},{rdi},~{rcx},~{r11},~{memory}"(i64 60, i64 %exit_code_13)
  unreachable
  ret i64 0
}
