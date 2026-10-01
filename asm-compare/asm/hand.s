hand_checksum_received:
	mov	ecx, dword ptr [rdi]
	mov	edx, ecx
	movzx	esi, cl
	movzx	eax, ch
	shr	ecx, 16
	shr	edx, 24
	add	esi, edx
	movzx	ecx, cl
	movzx	edx, word ptr [rdi + 4]
	rol	dx, 8
	movzx	edx, dx
	add	eax, esi
	add	eax, ecx
	add	eax, edx
	ret

hand_checksum_routed:
	mov	ecx, dword ptr [rdi]
	mov	edx, ecx
	movzx	esi, cl
	movzx	eax, ch
	shr	ecx, 16
	shr	edx, 24
	add	esi, edx
	movzx	ecx, cl
	movzx	edx, word ptr [rdi + 4]
	rol	dx, 8
	movzx	edx, dx
	add	eax, esi
	add	eax, ecx
	add	eax, edx
	ret

hand_pipeline:
	rol	si, 8
	movzx	eax, dl
	shl	rax, 48
	movzx	ecx, si
	shl	rcx, 32
	or	rcx, rax
	mov	eax, edi
	or	rax, rcx
	ret

hand_morph:  # alias of groups_morph, LLVM merged identical IR
	movabs	rax, 71776123356184575
	and	rax, rdi
	shr	rdi, 32
	rol	di, 8
	movzx	ecx, di
	shl	rcx, 32
	or	rax, rcx
	ret

hand_transmute:  # alias of groups_transmute, LLVM merged identical IR
	mov	rax, rdi
	ret

hand_transmute_mut:  # alias of groups_transmute_mut, LLVM merged identical IR
	mov	rax, rdi
	ret

hand_transmute_ref:  # alias of groups_transmute_ref, LLVM merged identical IR
	mov	rax, rdi
	ret
