groups_checksum_received:
	mov	ecx, dword ptr [rdi]
	mov	edx, ecx
	movzx	esi, cl
	movzx	eax, ch
	movzx	edi, word ptr [rdi + 4]
	shr	ecx, 16
	shr	edx, 24
	add	esi, edx
	movzx	ecx, cl
	rol	di, 8
	movzx	edx, di
	add	eax, esi
	add	eax, ecx
	add	eax, edx
	ret

groups_checksum_routed:
	mov	ecx, dword ptr [rdi]
	mov	edx, ecx
	movzx	esi, cl
	movzx	eax, ch
	movzx	edi, word ptr [rdi + 4]
	shr	ecx, 16
	shr	edx, 24
	add	esi, edx
	movzx	ecx, cl
	rol	di, 8
	movzx	edx, di
	add	eax, esi
	add	eax, ecx
	add	eax, edx
	ret

groups_morph:
	movabs	rax, 71776123356184575
	and	rax, rdi
	shr	rdi, 32
	rol	di, 8
	movzx	ecx, di
	shl	rcx, 32
	or	rax, rcx
	ret

groups_pipeline:
	movzx	eax, dl
	shl	rax, 48
	mov	ecx, edi
	or	rcx, rax
	rol	si, 8
	movzx	eax, si
	shl	rax, 32
	or	rax, rcx
	ret

groups_transmute:
	mov	rax, rdi
	ret

groups_transmute_mut:
	mov	rax, rdi
	ret

groups_transmute_ref:
	mov	rax, rdi
	ret
