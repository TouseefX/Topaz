// BytecodeBuilder -- 0.735 Studio, Mac symbols, 2026-08
// tier 1, 85 functions


// Address: 0x10316ad00 | Name: __ZNK4Luau15BytecodeBuilder9StringRefeqERKS1_
bool __fastcall Luau::BytecodeBuilder::StringRef::operator==(__int64 a1, __int64 a2)
{
  const void *v4; // rdi
  const void *v5; // rsi
  size_t v7; // rdx

  v4 = *(const void **)a1; /*0x10316ad06*/
  v5 = *(const void **)a2; /*0x10316ad0f*/
  if ( v4 == nullptr || v5 == nullptr ) /*0x10316ad15*/
    return v4 == v5; /*0x10316ad21*/
  v7 = *(_QWORD *)(a1 + 8); /*0x10316ad25*/
  return v7 == *(_QWORD *)(a2 + 8) && memcmp(__s1: v4, __s2: v5, __n: v7) == 0; /*0x10316ad24*/
}

// Address: 0x10316ad42 | Name: __ZNK4Luau15BytecodeBuilder10TableShapeeqERKS1_
bool __fastcall Luau::BytecodeBuilder::TableShape::operator==(__int64 a1, __int64 a2)
{
  __int64 v2; // r14
  size_t v5; // r14
  unsigned __int8 v7; // cl
  bool v8; // al

  v2 = *(unsigned int *)(a1 + 256); /*0x10316ad4c*/
  if ( (_DWORD)v2 != *(_DWORD *)(a2 + 256) ) /*0x10316ad5a*/
    return false; /*0x10316ad5a*/
  v5 = 4 * v2; /*0x10316ad62*/
  if ( memcmp(__s1: (const void *)a1, __s2: (const void *)a2, __n: v5) != 0 ) /*0x10316ad70*/
    return false; /*0x10316ad72*/
  v7 = *(_BYTE *)(a1 + 260); /*0x10316ad7f*/
  v8 = v7 == *(_BYTE *)(a2 + 260); /*0x10316ad8c*/
  if ( (v7 & v8) == 1 ) /*0x10316ad96*/
    return memcmp(__s1: (const void *)(a1 + 128), __s2: (const void *)(a2 + 128), __n: v5) == 0; /*0x10316adb3*/
  else
    return (v7 ^ 1) & v8; /*0x10316adbb*/
}

// Address: 0x10316adc0 | Name: __ZNK4Luau15BytecodeBuilder13StringRefHashclERKNS0_9StringRefE
__int64 __fastcall Luau::BytecodeBuilder::StringRefHash::operator()(__int64 a1, __int64 a2, unsigned __int64 a3)
{
  return Luau::hashRange(this: *(Luau **)a2, a2: *(const char **)(a2 + 8), a3); /*0x10316adcb*/
}

// Address: 0x10316add2 | Name: __ZNK4Luau15BytecodeBuilder15ConstantKeyHashclERKNS0_11ConstantKeyE
__int64 __fastcall Luau::BytecodeBuilder::ConstantKeyHash::operator()(__int64 a1, int *a2)
{
  int v2; // eax
  __m128i v3; // xmm0
  __m128i v4; // xmm1
  __m128i v5; // xmm0
  unsigned int v7; // eax
  unsigned int v8; // ecx

  v2 = *a2; /*0x10316add2*/
  if ( *a2 == 5 ) /*0x10316add7*/
    return (73856093 * ((unsigned int)*((_QWORD *)a2 + 1) ^ HIDWORD(*((_QWORD *)a2 + 1)))) /*0x10316ae69*/
         ^ (19349663 * ((unsigned int)*((_QWORD *)a2 + 2) ^ HIDWORD(*((_QWORD *)a2 + 2))))
         ^ (83492791 * ((unsigned int)*((_QWORD *)a2 + 3) ^ HIDWORD(*((_QWORD *)a2 + 3))))
         ^ (39916801 * ((unsigned int)*((_QWORD *)a2 + 4) ^ HIDWORD(*((_QWORD *)a2 + 4))));
  if ( v2 == 4 ) /*0x10316addc*/
  {
    v3 = _mm_loadu_si128((const __m128i *)(a2 + 2)); /*0x10316ade6*/
    v4 = _mm_mullo_epi32(_mm_xor_si128(_mm_srli_epi32(v3, 0x11u), v3), (__m128i)xmmword_1094CC2A0); /*0x10316adf8*/
    v5 = _mm_xor_si128(_mm_shuffle_epi32(v4, 238), v4); /*0x10316ae06*/
    return (unsigned int)_mm_cvtsi128_si32(_mm_xor_si128(_mm_shuffle_epi32(v5, 85), v5)); /*0x10316ae13*/
  }
  else
  {
    v7 = HIDWORD(*((_QWORD *)a2 + 1)) ^ (1540483477 * v2); /*0x10316ae7d*/
    v8 = 1540483477 * (*((_QWORD *)a2 + 1) ^ (v7 >> 18)); /*0x10316ae86*/
    return 1540483477 /*0x10316aeab*/
         * ((1540483477 * (v7 ^ (v8 >> 22))) ^ ((1540483477 * (v8 ^ ((1540483477 * (v7 ^ (v8 >> 22))) >> 17))) >> 19));
  }
}

// Address: 0x10316aeb2 | Name: __ZNK4Luau15BytecodeBuilder14TableShapeHashclERKNS0_10TableShapeE
__int64 __fastcall Luau::BytecodeBuilder::TableShapeHash::operator()(__int64 a1, __int64 a2)
{
  unsigned int v2; // ecx
  __int64 v3; // rdi

  if ( *(_DWORD *)(a2 + 256) == 0 ) /*0x10316aebf*/
    return 2166136261LL; /*0x10316aef4*/
  v2 = -2128831035; /*0x10316aec7*/
  v3 = 0; /*0x10316aecc*/
  do /*0x10316aeee*/
  {
    v2 = 16777619 * (*(_DWORD *)(a2 + 4 * v3) ^ v2); /*0x10316aed1*/
    if ( *(_BYTE *)(a2 + 260) != 0 ) /*0x10316aed9*/
      v2 = 16777619 * (*(_DWORD *)(a2 + 4 * v3 + 128) ^ v2); /*0x10316aee2*/
    ++v3; /*0x10316aee8*/
  }
  while ( *(_DWORD *)(a2 + 256) != v3 ); /*0x10316aeee*/
  return v2; /*0x10316aef9*/
}

// Address: 0x10316aefc | Name: __ZN4Luau15BytecodeBuilderC2EPNS_15BytecodeEncoderE
__int64 __fastcall Luau::BytecodeBuilder::BytecodeBuilder(__int64 a1, __int64 a2)
{
  __int64 v3; // [rsp+50h] [rbp-50h]
  __int64 v4; // [rsp+58h] [rbp-48h]
  __int64 v5; // [rsp+60h] [rbp-40h]
  __int64 v6; // [rsp+68h] [rbp-38h]

  *(_QWORD *)a1 = off_10C8C02F0; /*0x10316af17*/
  v6 = a1 + 8; /*0x10316af1e*/
  *(_OWORD *)(a1 + 8) = 0; /*0x10316af25*/
  *(_QWORD *)(a1 + 24) = 0; /*0x10316af2b*/
  *(_QWORD *)(a1 + 32) = -1; /*0x10316af2f*/
  v3 = a1 + 72; /*0x10316af3f*/
  v4 = a1 + 96; /*0x10316af47*/
  v5 = a1 + 120; /*0x10316af4f*/
  *(_OWORD *)(a1 + 40) = 0; /*0x10316af6f*/
  *(_OWORD *)(a1 + 56) = 0; /*0x10316af74*/
  *(_OWORD *)(a1 + 72) = 0; /*0x10316af79*/
  *(_OWORD *)(a1 + 88) = 0; /*0x10316af7e*/
  *(_OWORD *)(a1 + 104) = 0; /*0x10316af83*/
  *(_OWORD *)(a1 + 120) = 0; /*0x10316af88*/
  *(_OWORD *)(a1 + 136) = 0; /*0x10316af8d*/
  *(_OWORD *)(a1 + 152) = 0; /*0x10316af95*/
  *(_OWORD *)(a1 + 168) = 0; /*0x10316af9d*/
  *(_OWORD *)(a1 + 184) = 0; /*0x10316afa5*/
  *(_OWORD *)(a1 + 200) = 0; /*0x10316afad*/
  *(_OWORD *)(a1 + 216) = 0; /*0x10316afb5*/
  *(_OWORD *)(a1 + 225) = 0; /*0x10316afbd*/
  *(_OWORD *)(a1 + 280) = 0; /*0x10316afc5*/
  *(_OWORD *)(a1 + 264) = 0; /*0x10316afcd*/
  *(_OWORD *)(a1 + 248) = 0; /*0x10316afd5*/
  *(_BYTE *)(a1 + 296) = 64; /*0x10316afdf*/
  *(_OWORD *)(a1 + 336) = 0; /*0x10316aff1*/
  *(_OWORD *)(a1 + 320) = 0; /*0x10316aff9*/
  *(_OWORD *)(a1 + 304) = 0; /*0x10316b001*/
  *(_BYTE *)(a1 + 352) = 64; /*0x10316b009*/
  *(_OWORD *)(a1 + 392) = 0; /*0x10316b01b*/
  *(_OWORD *)(a1 + 376) = 0; /*0x10316b023*/
  *(_OWORD *)(a1 + 360) = 0; /*0x10316b02b*/
  *(_BYTE *)(a1 + 408) = 64; /*0x10316b033*/
  *(_DWORD *)(a1 + 416) = 0; /*0x10316b03a*/
  *(_OWORD *)(a1 + 424) = 0; /*0x10316b045*/
  *(_OWORD *)(a1 + 440) = 0; /*0x10316b04d*/
  *(_OWORD *)(a1 + 456) = 0; /*0x10316b055*/
  *(_OWORD *)(a1 + 472) = 0; /*0x10316b05d*/
  *(_OWORD *)(a1 + 488) = 0; /*0x10316b065*/
  *(_OWORD *)(a1 + 504) = 0; /*0x10316b06d*/
  *(_OWORD *)(a1 + 520) = 0; /*0x10316b075*/
  *(_OWORD *)(a1 + 536) = 0; /*0x10316b07d*/
  *(_OWORD *)(a1 + 552) = 0; /*0x10316b085*/
  *(_OWORD *)(a1 + 568) = 0; /*0x10316b08d*/
  *(_QWORD *)(a1 + 584) = 0; /*0x10316b095*/
  *(_BYTE *)(a1 + 592) = 64; /*0x10316b09c*/
  *(_QWORD *)(a1 + 664) = 0; /*0x10316b0ae*/
  *(_OWORD *)(a1 + 648) = 0; /*0x10316b0b5*/
  *(_OWORD *)(a1 + 632) = 0; /*0x10316b0bd*/
  *(_OWORD *)(a1 + 616) = 0; /*0x10316b0c5*/
  *(_OWORD *)(a1 + 600) = 0; /*0x10316b0cd*/
  *(_QWORD *)(a1 + 672) = a2; /*0x10316b0d5*/
  *(_OWORD *)(a1 + 692) = 0; /*0x10316b0e7*/
  *(_OWORD *)(a1 + 680) = 0; /*0x10316b0ef*/
  *(_QWORD *)(a1 + 792) = 0; /*0x10316b0f7*/
  *(_OWORD *)(a1 + 776) = 0; /*0x10316b109*/
  *(_OWORD *)(a1 + 760) = 0; /*0x10316b111*/
  *(_OWORD *)(a1 + 744) = 0; /*0x10316b119*/
  *(_OWORD *)(a1 + 728) = 0; /*0x10316b121*/
  *(_OWORD *)(a1 + 712) = 0; /*0x10316b129*/
  std::vector<unsigned int>::reserve(a1: a1 + 48, a2: 32); /*0x10316b159*/
  std::vector<int>::reserve(a1: v3, a2: 32); /*0x10316b167*/
  std::vector<Luau::BytecodeBuilder::Constant>::reserve(a1: v4, a2: 16); /*0x10316b175*/
  std::vector<unsigned int>::reserve(a1: v5, a2: 16); /*0x10316b183*/
  return std::vector<Luau::BytecodeBuilder::Function>::reserve(a1: v6, a2: 8); /*0x10316b196*/
}

// Address: 0x10316b524 | Name: __ZN4Luau15BytecodeBuilderC1EPNS_15BytecodeEncoderE
__int64 __fastcall Luau::BytecodeBuilder::BytecodeBuilder(__int64 a1, __int64 a2)
{
  return Luau::BytecodeBuilder::BytecodeBuilder(a1, a2); /*0x10316b528*/
}

// Address: 0x10316b52e | Name: __ZN4Luau15BytecodeBuilder13beginFunctionEhb
__int64 __fastcall Luau::BytecodeBuilder::beginFunction(Luau::BytecodeBuilder *this, char a2, char a3)
{
  __int64 v4; // r15
  __int64 v5; // rbx
  unsigned int v6; // ebx
  __int128 v8; // [rsp+0h] [rbp-A0h] BYREF
  __int128 v9; // [rsp+10h] [rbp-90h]
  int v10; // [rsp+20h] [rbp-80h]
  __int128 v11; // [rsp+28h] [rbp-78h]
  __int128 v12; // [rsp+38h] [rbp-68h]
  __int128 v13; // [rsp+48h] [rbp-58h]
  __int128 v14; // [rsp+58h] [rbp-48h]
  __int128 v15; // [rsp+68h] [rbp-38h]
  __int128 v16; // [rsp+78h] [rbp-28h]

  v4 = *((_QWORD *)this + 1); /*0x10316b545*/
  v5 = *((_QWORD *)this + 2); /*0x10316b549*/
  v9 = 0; /*0x10316b557*/
  v8 = 0; /*0x10316b55b*/
  v10 = 0; /*0x10316b55e*/
  v11 = 0; /*0x10316b565*/
  v12 = 0; /*0x10316b569*/
  v13 = 0; /*0x10316b56d*/
  v14 = 0; /*0x10316b571*/
  v15 = 0; /*0x10316b575*/
  v16 = 0; /*0x10316b579*/
  BYTE9(v9) = a2; /*0x10316b57d*/
  BYTE11(v9) = a3; /*0x10316b581*/
  std::vector<Luau::BytecodeBuilder::Function>::push_back[abi:ne200100](a1: (char *)this + 8, a2: &v8); /*0x10316b587*/
  v6 = -252645135 * ((unsigned __int64)(v5 - v4) >> 3); /*0x10316b593*/
  *((_DWORD *)this + 8) = v6; /*0x10316b599*/
  *((_BYTE *)this + 240) = 0; /*0x10316b59d*/
  *((_DWORD *)this + 104) = 0; /*0x10316b5a5*/
  if ( (BYTE8(v15) & 1) != 0 ) /*0x10316b5b4*/
    operator delete(a1: *((void **)&v16 + 1)); /*0x10316b5ba*/
  if ( (_QWORD)v14 != 0 ) /*0x10316b5c6*/
  {
    *((_QWORD *)&v14 + 1) = v14; /*0x10316b5c8*/
    operator delete(a1: (void *)v14); /*0x10316b5cc*/
  }
  if ( (BYTE8(v12) & 1) != 0 ) /*0x10316b5d5*/
    operator delete(a1: *((void **)&v13 + 1)); /*0x10316b5db*/
  if ( (v11 & 1) != 0 ) /*0x10316b5e4*/
    operator delete(a1: (void *)v12); /*0x10316b5ea*/
  if ( (v8 & 1) != 0 ) /*0x10316b5f6*/
    operator delete(a1: (void *)v9); /*0x10316b5ff*/
  return v6; /*0x10316b606*/
}

// Address: 0x10316b6dc | Name: __ZN4Luau15BytecodeBuilder10clearStateEv
_BYTE *__fastcall Luau::BytecodeBuilder::clearState(Luau::BytecodeBuilder *this)
{
  __int64 v2; // rcx
  _BYTE *result; // rax

  v2 = *((_QWORD *)this + 9); /*0x10316b6e9*/
  *((_QWORD *)this + 7) = *((_QWORD *)this + 6); /*0x10316b6ed*/
  *((_QWORD *)this + 10) = v2; /*0x10316b6f1*/
  *((_QWORD *)this + 13) = *((_QWORD *)this + 12); /*0x10316b6f9*/
  *((_QWORD *)this + 16) = *((_QWORD *)this + 15); /*0x10316b701*/
  *((_QWORD *)this + 19) = *((_QWORD *)this + 18); /*0x10316b70f*/
  *((_QWORD *)this + 28) = *((_QWORD *)this + 27); /*0x10316b71d*/
  *((_QWORD *)this + 22) = *((_QWORD *)this + 21); /*0x10316b72b*/
  *((_QWORD *)this + 54) = *((_QWORD *)this + 53); /*0x10316b739*/
  *((_QWORD *)this + 57) = *((_QWORD *)this + 56); /*0x10316b747*/
  *((_QWORD *)this + 60) = *((_QWORD *)this + 59); /*0x10316b755*/
  *((_QWORD *)this + 63) = *((_QWORD *)this + 62); /*0x10316b763*/
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::ConstantKey,std::pair<Luau::BytecodeBuilder::ConstantKey,int>,std::pair<Luau::BytecodeBuilder::ConstantKey const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::ConstantKey,int>,Luau::BytecodeBuilder::ConstantKeyHash,std::equal_to<Luau::BytecodeBuilder::ConstantKey>>::clear( /*0x10316b776*/
    a1: (char *)this + 248,
    a2: 32);
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::TableShape,std::pair<Luau::BytecodeBuilder::TableShape,int>,std::pair<Luau::BytecodeBuilder::TableShape const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::TableShape,int>,Luau::BytecodeBuilder::TableShapeHash,std::equal_to<Luau::BytecodeBuilder::TableShape>>::clear( /*0x10316b787*/
    a1: (char *)this + 304,
    a2: 32);
  Luau::detail::DenseHashTable2<unsigned int,std::pair<unsigned int,short>,std::pair<unsigned int const,short>,Luau::detail::ItemInterfaceMap2<unsigned int,short>,std::hash<unsigned int>,std::equal_to<unsigned int>>::clear( /*0x10316b798*/
    a1: (char *)this + 360,
    a2: 32);
  result = *((_BYTE **)this + 78); /*0x10316b79d*/
  *((_QWORD *)this + 79) = result; /*0x10316b7a4*/
  if ( (*((_BYTE *)this + 648) & 1) != 0 ) /*0x10316b7b2*/
  {
    result = *((_BYTE **)this + 83); /*0x10316b7bf*/
    *result = 0; /*0x10316b7c6*/
    *((_QWORD *)this + 82) = 0; /*0x10316b7c9*/
  }
  else
  {
    *((_WORD *)this + 324) = 0; /*0x10316b7b4*/
  }
  return result; /*0x10316b7d8*/
}

// Address: 0x10316b7dc | Name: __ZN4Luau15BytecodeBuilder11endFunctionEhhhy
_BYTE *__fastcall Luau::BytecodeBuilder::endFunction(
        Luau::BytecodeBuilder *this,
        char a2,
        char a3,
        unsigned __int8 a4,
        __int64 a5)
{
  __int64 v8; // r12
  char *v9; // rax
  _QWORD *v10; // rsi
  __int64 v11; // rdi
  __int64 v12; // rax
  _BYTE *result; // rax
  __int64 v14; // rcx
  __int128 v15; // [rsp+0h] [rbp-40h] BYREF
  __int64 v16; // [rsp+10h] [rbp-30h]

  v8 = *((_QWORD *)this + 1) + 136LL * *((unsigned int *)this + 8); /*0x10316b808*/
  *(_BYTE *)(v8 + 24) = a2; /*0x10316b80c*/
  *(_BYTE *)(v8 + 26) = a3; /*0x10316b811*/
  v9 = *((char **)this + 98); /*0x10316b815*/
  if ( v9 != nullptr ) /*0x10316b81f*/
  {
    v10 = (_QWORD *)((char *)this + *((_QWORD *)this + 99)); /*0x10316b828*/
    if ( ((unsigned __int8)v9 & 1) != 0 ) /*0x10316b82d*/
      v9 = *(char **)&v9[*v10 - 1]; /*0x10316b832*/
    ((void (__fastcall *)(__int128 *, _QWORD *, __int64))v9)(a1: &v15, a2: v10, a3: v8 + 88); /*0x10316b840*/
    if ( (*(_BYTE *)(v8 + 40) & 1) != 0 ) /*0x10316b84d*/
      operator delete(a1: *(void **)(v8 + 56)); /*0x10316b854*/
    *(_QWORD *)(v8 + 56) = v16; /*0x10316b85d*/
    *(_OWORD *)(v8 + 40) = v15; /*0x10316b865*/
  }
  std::string::reserve( /*0x10316b886*/
    a1: v8,
    a2: 2LL * (*((_QWORD *)this + 7) - *((_QWORD *)this + 6))
  - ((__int64)(*((_QWORD *)this + 7) - *((_QWORD *)this + 6)) >> 2)
  + 32);
  v11 = *((_QWORD *)this + 84); /*0x10316b88b*/
  if ( v11 != 0 ) /*0x10316b895*/
    (*(void (__fastcall **)(__int64, _QWORD, __int64))(*(_QWORD *)v11 + 16LL))( /*0x10316b8a9*/
      a1: v11,
      a2: *((_QWORD *)this + 6),
      a3: (__int64)(*((_QWORD *)this + 7) - *((_QWORD *)this + 6)) >> 2);
  Luau::BytecodeBuilder::writeFunction(a1: this, a2: v8, a3: *((unsigned int *)this + 8), a4, a5); /*0x10316b8bc*/
  *((_DWORD *)this + 8) = -1; /*0x10316b8c1*/
  v12 = *((_QWORD *)this + 6); /*0x10316b8c8*/
  *((_QWORD *)this + 5) += (*((_QWORD *)this + 7) - v12) >> 2; /*0x10316b8d7*/
  if ( (_BYTE)FFlag::LuauVirtualBcBuilder == 1 ) /*0x10316b8e2*/
    return Luau::BytecodeBuilder::clearState(this); /*0x10316b8e7*/
  *((_QWORD *)this + 7) = v12; /*0x10316b8f1*/
  v14 = *((_QWORD *)this + 12); /*0x10316b8f9*/
  *((_QWORD *)this + 10) = *((_QWORD *)this + 9); /*0x10316b8fd*/
  *((_QWORD *)this + 13) = v14; /*0x10316b901*/
  *((_QWORD *)this + 16) = *((_QWORD *)this + 15); /*0x10316b909*/
  *((_QWORD *)this + 19) = *((_QWORD *)this + 18); /*0x10316b917*/
  *((_QWORD *)this + 28) = *((_QWORD *)this + 27); /*0x10316b925*/
  *((_QWORD *)this + 22) = *((_QWORD *)this + 21); /*0x10316b933*/
  *((_QWORD *)this + 54) = *((_QWORD *)this + 53); /*0x10316b941*/
  *((_QWORD *)this + 57) = *((_QWORD *)this + 56); /*0x10316b94f*/
  *((_QWORD *)this + 60) = *((_QWORD *)this + 59); /*0x10316b95d*/
  *((_QWORD *)this + 63) = *((_QWORD *)this + 62); /*0x10316b96b*/
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::ConstantKey,std::pair<Luau::BytecodeBuilder::ConstantKey,int>,std::pair<Luau::BytecodeBuilder::ConstantKey const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::ConstantKey,int>,Luau::BytecodeBuilder::ConstantKeyHash,std::equal_to<Luau::BytecodeBuilder::ConstantKey>>::clear( /*0x10316b97e*/
    a1: (char *)this + 248,
    a2: 32);
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::TableShape,std::pair<Luau::BytecodeBuilder::TableShape,int>,std::pair<Luau::BytecodeBuilder::TableShape const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::TableShape,int>,Luau::BytecodeBuilder::TableShapeHash,std::equal_to<Luau::BytecodeBuilder::TableShape>>::clear( /*0x10316b98f*/
    a1: (char *)this + 304,
    a2: 32);
  Luau::detail::DenseHashTable2<unsigned int,std::pair<unsigned int,short>,std::pair<unsigned int const,short>,Luau::detail::ItemInterfaceMap2<unsigned int,short>,std::hash<unsigned int>,std::equal_to<unsigned int>>::clear( /*0x10316b9a0*/
    a1: (char *)this + 360,
    a2: 32);
  result = *((_BYTE **)this + 78); /*0x10316b9a5*/
  *((_QWORD *)this + 79) = result; /*0x10316b9ac*/
  if ( (*((_BYTE *)this + 648) & 1) != 0 ) /*0x10316b9ba*/
  {
    result = *((_BYTE **)this + 83); /*0x10316b9c7*/
    *result = 0; /*0x10316b9ce*/
    *((_QWORD *)this + 82) = 0; /*0x10316b9d1*/
  }
  else
  {
    *((_WORD *)this + 324) = 0; /*0x10316b9bc*/
  }
  return result; /*0x10316b9dc*/
}

// Address: 0x10316b9ec | Name: __ZN4Luau15BytecodeBuilder13writeFunctionERNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEEjhy
__int16 *__fastcall Luau::BytecodeBuilder::writeFunction(
        __int64 a1,
        __int64 a2,
        unsigned int a3,
        char a4,
        unsigned __int64 a5)
{
  unsigned int v7; // eax
  __int64 v8; // rax
  __int64 v9; // rcx
  __int64 v10; // r13
  unsigned int v11; // ebx
  unsigned __int64 v12; // rbx
  unsigned __int8 *v13; // r13
  unsigned __int64 v14; // r12
  bool v15; // cc
  unsigned __int64 v16; // rbx
  unsigned __int64 v17; // r12
  unsigned __int64 v18; // rbx
  unsigned __int64 v19; // r12
  unsigned int v20; // edx
  __int64 v21; // rsi
  __int64 v22; // rdx
  _BYTE *v23; // rbx
  _BYTE *i; // r12
  __int64 v25; // rbx
  unsigned __int64 v26; // r12
  unsigned __int64 v27; // rbx
  unsigned __int64 v28; // rbx
  unsigned __int64 v29; // r12
  unsigned int v30; // ebx
  unsigned __int64 v31; // rbx
  unsigned __int64 v32; // r12
  unsigned int v33; // edx
  float *v34; // rsi
  __int64 v35; // rdx
  unsigned __int64 v36; // rbx
  unsigned __int64 v37; // r12
  float *v38; // rbx
  float *j; // r12
  unsigned __int64 v40; // rbx
  unsigned __int64 v41; // r12
  __int64 v42; // r12
  float v43; // xmm0_4
  unsigned __int64 v44; // rbx
  unsigned __int64 v45; // r15
  bool v46; // cf
  __int64 v47; // xmm0_8
  unsigned __int64 v48; // rbx
  unsigned __int64 v49; // r15
  __int64 v50; // rdx
  unsigned __int64 v51; // rbx
  unsigned __int64 v52; // r15
  __int64 v53; // rdi
  __int64 v54; // rbx
  unsigned __int64 v55; // rbx
  unsigned __int64 v56; // r15
  __int64 v57; // r15
  unsigned __int64 v58; // rbx
  unsigned __int64 v59; // r12
  unsigned __int64 v60; // r15
  float v61; // xmm0_4
  float v62; // xmm0_4
  float v63; // xmm0_4
  unsigned __int64 v64; // rbx
  unsigned __int64 v65; // r15
  __int64 v66; // rcx
  unsigned __int64 v67; // rbx
  unsigned __int64 v68; // r15
  unsigned __int64 v69; // r12
  unsigned __int64 v70; // rbx
  unsigned __int64 v71; // r15
  unsigned __int64 v72; // rbx
  unsigned __int64 v73; // r12
  unsigned int *v74; // rbx
  unsigned __int64 v75; // r12
  unsigned __int64 v76; // r13
  unsigned __int64 v77; // rbx
  unsigned __int64 v78; // r12
  unsigned __int64 v79; // rbx
  unsigned __int64 v80; // r12
  _DWORD *m; // rax
  unsigned __int64 v82; // rbx
  unsigned __int64 v83; // r12
  unsigned int *v84; // r15
  unsigned __int64 v85; // r12
  unsigned __int64 v86; // rbx
  unsigned __int64 v87; // rbx
  unsigned __int64 v88; // r12
  unsigned __int64 v89; // rbx
  unsigned __int64 v90; // r12
  unsigned __int64 v91; // rbx
  unsigned __int64 v92; // r12
  unsigned int *v93; // rbx
  unsigned __int64 v94; // r12
  unsigned __int64 v95; // r13
  unsigned __int64 v96; // rbx
  unsigned __int64 v97; // r12
  __int16 *v98; // rbx
  __int16 *result; // rax
  unsigned __int64 v100; // r12
  unsigned __int64 v101; // r13
  unsigned __int64 v102; // r12
  unsigned __int64 v103; // rbx
  __int64 v104; // [rsp+8h] [rbp-68h]
  __int64 v106; // [rsp+18h] [rbp-58h]
  __int64 v107; // [rsp+18h] [rbp-58h]
  char v108; // [rsp+24h] [rbp-4Ch]
  __int64 v109; // [rsp+28h] [rbp-48h]
  __int64 v111; // [rsp+38h] [rbp-38h]
  __int64 v112; // [rsp+38h] [rbp-38h]
  unsigned int *k; // [rsp+38h] [rbp-38h]
  unsigned int *n; // [rsp+38h] [rbp-38h]
  unsigned int *ii; // [rsp+38h] [rbp-38h]
  __int16 *jj; // [rsp+38h] [rbp-38h]
  float v117[12]; // [rsp+40h] [rbp-30h] BYREF

  v109 = *(_QWORD *)(a1 + 8) + 136LL * a3; /*0x10316ba20*/
  LOBYTE(v117[0]) = *(_BYTE *)(v109 + 24); /*0x10316ba2d*/
  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316ba3b*/
  LOBYTE(v117[0]) = *(_BYTE *)(v109 + 25); /*0x10316ba45*/
  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316ba53*/
  LOBYTE(v117[0]) = *(_BYTE *)(v109 + 26); /*0x10316ba5d*/
  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316ba6b*/
  LOBYTE(v117[0]) = *(_BYTE *)(v109 + 27); /*0x10316ba75*/
  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316ba83*/
  v108 = a4; /*0x10316ba88*/
  LOBYTE(v117[0]) = a4; /*0x10316ba8c*/
  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316ba9b*/
  v7 = *(unsigned __int8 *)(v109 + 112); /*0x10316baa0*/
  if ( (v7 & 1) != 0 ) /*0x10316baa8*/
  {
    v9 = v109; /*0x10316bab2*/
    v8 = *(_QWORD *)(v109 + 120); /*0x10316bab6*/
  }
  else
  {
    v8 = v7 >> 1; /*0x10316baaa*/
    v9 = v109; /*0x10316baac*/
  }
  v10 = a1; /*0x10316babd*/
  if ( v8 == 0 && *(_QWORD *)(a1 + 496) == *(_QWORD *)(a1 + 504) && *(_QWORD *)(a1 + 472) == *(_QWORD *)(a1 + 480) ) /*0x10316bae1*/
  {
    v34 = v117; /*0x10316c99d*/
    LOBYTE(v117[0]) = 0; /*0x10316c9a1*/
    v35 = 1; /*0x10316c9a4*/
  }
  else
  {
    if ( (*(_BYTE *)(a1 + 760) & 1) != 0 ) /*0x10316baef*/
    {
      **(_BYTE **)(a1 + 776) = 0; /*0x10316bb04*/
      *(_QWORD *)(a1 + 768) = 0; /*0x10316bb07*/
    }
    else
    {
      *(_WORD *)(a1 + 760) = 0; /*0x10316baf1*/
    }
    v11 = *(unsigned __int8 *)(v9 + 112); /*0x10316bb12*/
    if ( (v11 & 1) != 0 ) /*0x10316bb19*/
      v12 = *(unsigned int *)(v9 + 120); /*0x10316bb1f*/
    else
      v12 = v11 >> 1; /*0x10316bb1b*/
    v13 = (unsigned __int8 *)(a1 + 760); /*0x10316bb22*/
    v14 = v12; /*0x10316bb29*/
    do /*0x10316bb5d*/
    {
      LOBYTE(v117[0]) = v12 & 0x7F | ((v12 > 0x7F) << 7); /*0x10316bb3f*/
      std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bb4d*/
      v14 >>= 7; /*0x10316bb52*/
      v15 = v12 <= 0x7F; /*0x10316bb56*/
      v12 = v14; /*0x10316bb5a*/
    }
    while ( !v15 ); /*0x10316bb5d*/
    v16 = (unsigned int)((*(_QWORD *)(a1 + 504) - *(_QWORD *)(a1 + 496)) >> 2); /*0x10316bb75*/
    v17 = v16; /*0x10316bb7b*/
    do /*0x10316bbaf*/
    {
      LOBYTE(v117[0]) = v16 & 0x7F | ((v16 > 0x7F) << 7); /*0x10316bb91*/
      std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bb9f*/
      v17 >>= 7; /*0x10316bba4*/
      v15 = v16 <= 0x7F; /*0x10316bba8*/
      v16 = v17; /*0x10316bbac*/
    }
    while ( !v15 ); /*0x10316bbaf*/
    v18 = (unsigned int)((*(_QWORD *)(a1 + 480) - *(_QWORD *)(a1 + 472)) >> 4); /*0x10316bbc7*/
    v19 = v18; /*0x10316bbcd*/
    do /*0x10316bc01*/
    {
      LOBYTE(v117[0]) = v18 & 0x7F | ((v18 > 0x7F) << 7); /*0x10316bbe3*/
      std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bbf1*/
      v19 >>= 7; /*0x10316bbf6*/
      v15 = v18 <= 0x7F; /*0x10316bbfa*/
      v18 = v19; /*0x10316bbfe*/
    }
    while ( !v15 ); /*0x10316bc01*/
    v20 = *(unsigned __int8 *)(v109 + 112); /*0x10316bc07*/
    if ( (v20 & 1) != 0 ) /*0x10316bc0e*/
    {
      v21 = *(_QWORD *)(v109 + 128); /*0x10316bc10*/
      v22 = *(_QWORD *)(v109 + 120); /*0x10316bc17*/
    }
    else
    {
      v21 = v109 + 113; /*0x10316bc1d*/
      v22 = v20 >> 1; /*0x10316bc21*/
    }
    std::string::append(a1: v13, a2: v21, a3: v22); /*0x10316bc26*/
    v23 = *(_BYTE **)(a1 + 496); /*0x10316bc2f*/
    for ( i = *(_BYTE **)(a1 + 504); v23 != i; v23 += 4 ) /*0x10316bc40*/
    {
      LOBYTE(v117[0]) = *v23; /*0x10316bc48*/
      std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bc56*/
    }
    v25 = *(_QWORD *)(a1 + 472); /*0x10316bc68*/
    v106 = *(_QWORD *)(a1 + 480); /*0x10316bc76*/
    if ( v25 != v106 ) /*0x10316bc7d*/
    {
      do /*0x10316bd3d*/
      {
        LOBYTE(v117[0]) = *(_BYTE *)v25; /*0x10316bc89*/
        std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bc97*/
        LOBYTE(v117[0]) = *(_BYTE *)(v25 + 4); /*0x10316bc9f*/
        std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bcad*/
        v111 = v25; /*0x10316bcb2*/
        v26 = *(unsigned int *)(v25 + 8); /*0x10316bcb6*/
        v27 = v26; /*0x10316bcba*/
        do /*0x10316bcef*/
        {
          LOBYTE(v117[0]) = v26 & 0x7F | ((v26 > 0x7F) << 7); /*0x10316bcd1*/
          std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bcdf*/
          v27 >>= 7; /*0x10316bce4*/
          v15 = v26 <= 0x7F; /*0x10316bce8*/
          v26 = v27; /*0x10316bcec*/
        }
        while ( !v15 ); /*0x10316bcef*/
        v28 = (unsigned int)(*(_DWORD *)(v111 + 12) - *(_DWORD *)(v111 + 8)); /*0x10316bcf8*/
        v29 = (unsigned int)v28; /*0x10316bcfb*/
        do /*0x10316bd2f*/
        {
          LOBYTE(v117[0]) = v28 & 0x7F | ((v28 > 0x7F) << 7); /*0x10316bd11*/
          std::string::append(a1: v13, a2: v117, a3: 1); /*0x10316bd1f*/
          v29 >>= 7; /*0x10316bd24*/
          v15 = v28 <= 0x7F; /*0x10316bd28*/
          v28 = v29; /*0x10316bd2c*/
        }
        while ( !v15 ); /*0x10316bd2f*/
        v25 = v111 + 16; /*0x10316bd35*/
      }
      while ( v111 + 16 != v106 ); /*0x10316bd3d*/
    }
    v30 = *v13; /*0x10316bd43*/
    if ( (v30 & 1) != 0 ) /*0x10316bd4b*/
    {
      v10 = a1; /*0x10316bd55*/
      v31 = *(unsigned int *)(a1 + 768); /*0x10316bd59*/
    }
    else
    {
      v31 = v30 >> 1; /*0x10316bd4d*/
      v10 = a1; /*0x10316bd4f*/
    }
    v32 = v31; /*0x10316bd64*/
    do /*0x10316bd98*/
    {
      LOBYTE(v117[0]) = v31 & 0x7F | ((v31 > 0x7F) << 7); /*0x10316bd7a*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bd88*/
      v32 >>= 7; /*0x10316bd8d*/
      v15 = v31 <= 0x7F; /*0x10316bd91*/
      v31 = v32; /*0x10316bd95*/
    }
    while ( !v15 ); /*0x10316bd98*/
    v33 = *(unsigned __int8 *)(v10 + 760); /*0x10316bd9a*/
    if ( (v33 & 1) != 0 ) /*0x10316bda5*/
    {
      v34 = *(float **)(v10 + 776); /*0x10316bda7*/
      v35 = *(_QWORD *)(v10 + 768); /*0x10316bdae*/
    }
    else
    {
      v34 = (float *)(v10 + 761); /*0x10316bdb7*/
      v35 = v33 >> 1; /*0x10316bdbe*/
    }
  }
  std::string::append(a1: a2, a2: v34, a3: v35); /*0x10316bdc3*/
  v36 = (unsigned int)((*(_QWORD *)(v10 + 56) - *(_QWORD *)(v10 + 48)) >> 2); /*0x10316bdd4*/
  v37 = v36; /*0x10316bdda*/
  do /*0x10316be0e*/
  {
    LOBYTE(v117[0]) = v36 & 0x7F | ((v36 > 0x7F) << 7); /*0x10316bdf0*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bdfe*/
    v37 >>= 7; /*0x10316be03*/
    v15 = v36 <= 0x7F; /*0x10316be07*/
    v36 = v37; /*0x10316be0b*/
  }
  while ( !v15 ); /*0x10316be0e*/
  v38 = *(float **)(v10 + 48); /*0x10316be10*/
  for ( j = *(float **)(v10 + 56); v38 != j; ++v38 ) /*0x10316be1b*/
  {
    v117[0] = *v38; /*0x10316be23*/
    std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316be31*/
  }
  v40 = -858993459 * (unsigned int)((*(_QWORD *)(v10 + 104) - *(_QWORD *)(v10 + 96)) >> 3); /*0x10316be4b*/
  v41 = v40; /*0x10316be55*/
  do /*0x10316be89*/
  {
    LOBYTE(v117[0]) = v40 & 0x7F | ((v40 > 0x7F) << 7); /*0x10316be6b*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316be79*/
    v41 >>= 7; /*0x10316be7e*/
    v15 = v40 <= 0x7F; /*0x10316be82*/
    v40 = v41; /*0x10316be86*/
  }
  while ( !v15 ); /*0x10316be89*/
  v42 = *(_QWORD *)(v10 + 96); /*0x10316be8b*/
  v104 = *(_QWORD *)(v10 + 104); /*0x10316be93*/
  if ( v42 != v104 ) /*0x10316be9a*/
  {
    while ( 2 ) /*0x10316bec0*/
    {
      switch ( *(_DWORD *)v42 ) /*0x10316bec0*/
      {
        case 0: /*0x10316bec0*/
          LOBYTE(v117[0]) = 0; /*0x10316bec2*/
          goto LABEL_58; /*0x10316bec6*/
        case 1: /*0x10316bec0*/
          LOBYTE(v117[0]) = 1; /*0x10316c04e*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c05d*/
          LOBYTE(v117[0]) = *(_BYTE *)(v42 + 8); /*0x10316c067*/
LABEL_58:
          v50 = 1; /*0x10316c06a*/
          goto LABEL_78; /*0x10316c06f*/
        case 2: /*0x10316bec0*/
          LOBYTE(v117[0]) = 2; /*0x10316bf96*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bfa5*/
          v47 = *(_QWORD *)(v42 + 8); /*0x10316bfaa*/
          goto LABEL_61; /*0x10316bfb1*/
        case 3: /*0x10316bec0*/
          LOBYTE(v117[0]) = 9; /*0x10316bfb6*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bfc5*/
          if ( *(__int64 *)(v42 + 8) < 0 ) /*0x10316bfd0*/
          {
            LOBYTE(v117[0]) = 1; /*0x10316c3ee*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c3fd*/
            v70 = -*(_QWORD *)(v42 + 8); /*0x10316c404*/
            v71 = v70; /*0x10316c409*/
            do /*0x10316c440*/
            {
              LOBYTE(v117[0]) = v70 & 0x7F | ((v70 > 0x7F) << 7); /*0x10316c41f*/
              std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c42d*/
              v71 >>= 7; /*0x10316c432*/
              v46 = v70 < 0x80; /*0x10316c436*/
              v70 = v71; /*0x10316c43d*/
            }
            while ( !v46 ); /*0x10316c440*/
          }
          else
          {
            LOBYTE(v117[0]) = 0; /*0x10316bfd6*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bfe5*/
            v48 = *(_QWORD *)(v42 + 8); /*0x10316bfea*/
            v49 = v48; /*0x10316bfef*/
            do /*0x10316c026*/
            {
              LOBYTE(v117[0]) = v48 & 0x7F | ((v48 > 0x7F) << 7); /*0x10316c005*/
              std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c013*/
              v49 >>= 7; /*0x10316c018*/
              v46 = v48 < 0x80; /*0x10316c01c*/
              v48 = v49; /*0x10316c023*/
            }
            while ( !v46 ); /*0x10316c026*/
          }
          goto LABEL_79; /*0x10316c026*/
        case 4: /*0x10316bec0*/
          LOBYTE(v117[0]) = 7; /*0x10316becb*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316beda*/
          v117[0] = *(float *)(v42 + 8); /*0x10316bee6*/
          std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316bef6*/
          v117[0] = *(float *)(v42 + 12); /*0x10316bf02*/
          std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316bf12*/
          v117[0] = *(float *)(v42 + 16); /*0x10316bf1e*/
          std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316bf2e*/
          v43 = *(float *)(v42 + 20); /*0x10316bf33*/
          goto LABEL_76; /*0x10316bf3a*/
        case 5: /*0x10316bec0*/
          if ( (_BYTE)FFlag::LuauCompileEmitVectorDouble == 1 ) /*0x10316c07b*/
          {
            LOBYTE(v117[0]) = 11; /*0x10316c081*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c090*/
            *(_QWORD *)v117 = *(_QWORD *)(v42 + 8); /*0x10316c09c*/
            std::string::append(a1: a2, a2: v117, a3: 8); /*0x10316c0ac*/
            *(_QWORD *)v117 = *(_QWORD *)(v42 + 16); /*0x10316c0b8*/
            std::string::append(a1: a2, a2: v117, a3: 8); /*0x10316c0c8*/
            *(_QWORD *)v117 = *(_QWORD *)(v42 + 24); /*0x10316c0d4*/
            std::string::append(a1: a2, a2: v117, a3: 8); /*0x10316c0e4*/
            v47 = *(_QWORD *)(v42 + 32); /*0x10316c0e9*/
LABEL_61:
            *(_QWORD *)v117 = v47; /*0x10316c0f0*/
            v50 = 8; /*0x10316c0f5*/
          }
          else
          {
            LOBYTE(v117[0]) = 7; /*0x10316c296*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c2a5*/
            v61 = *(double *)(v42 + 8); /*0x10316c2ad*/
            v117[0] = v61; /*0x10316c2b4*/
            std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316c2c4*/
            v62 = *(double *)(v42 + 16); /*0x10316c2cc*/
            v117[0] = v62; /*0x10316c2d3*/
            std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316c2e3*/
            v63 = *(double *)(v42 + 24); /*0x10316c2eb*/
            v117[0] = v63; /*0x10316c2f2*/
            std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316c302*/
            v43 = *(double *)(v42 + 32); /*0x10316c30a*/
LABEL_76:
            v117[0] = v43; /*0x10316c311*/
LABEL_77:
            v50 = 4; /*0x10316c316*/
          }
LABEL_78:
          std::string::append(a1: a2, a2: v117, a3: v50); /*0x10316c31b*/
LABEL_79:
          v42 += 40; /*0x10316c326*/
          if ( v42 == v104 ) /*0x10316c32e*/
            break; /*0x10316c32e*/
          continue; /*0x10316c32e*/
        case 6: /*0x10316bec0*/
          LOBYTE(v117[0]) = 3; /*0x10316c0ff*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c10e*/
          v51 = *(unsigned int *)(v42 + 8); /*0x10316c113*/
          v52 = v51; /*0x10316c118*/
          do /*0x10316c14f*/
          {
            LOBYTE(v117[0]) = v51 & 0x7F | ((v51 > 0x7F) << 7); /*0x10316c12e*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c13c*/
            v52 >>= 7; /*0x10316c141*/
            v46 = v51 < 0x80; /*0x10316c145*/
            v51 = v52; /*0x10316c14c*/
          }
          while ( !v46 ); /*0x10316c14f*/
          goto LABEL_79; /*0x10316c14f*/
        case 7: /*0x10316bec0*/
          LOBYTE(v117[0]) = 4; /*0x10316c02d*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c03c*/
          v117[0] = *(float *)(v42 + 8); /*0x10316c046*/
          goto LABEL_77; /*0x10316c049*/
        case 8: /*0x10316bec0*/
          v54 = *(_QWORD *)(a1 + 168) + 264LL * *(unsigned int *)(v42 + 8); /*0x10316c1a6*/
          v107 = v42; /*0x10316c1b2*/
          v112 = v54; /*0x10316c1b6*/
          if ( *(_BYTE *)(v54 + 260) == 1 ) /*0x10316c1ba*/
          {
            LOBYTE(v117[0]) = 8; /*0x10316c1c0*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c1cf*/
            v55 = *(unsigned int *)(v54 + 256); /*0x10316c1d4*/
            v56 = v55; /*0x10316c1da*/
            do /*0x10316c20e*/
            {
              LOBYTE(v117[0]) = v55 & 0x7F | ((v55 > 0x7F) << 7); /*0x10316c1f0*/
              std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c1fe*/
              v56 >>= 7; /*0x10316c203*/
              v15 = v55 <= 0x7F; /*0x10316c207*/
              v55 = v56; /*0x10316c20b*/
            }
            while ( !v15 ); /*0x10316c20e*/
            v57 = v112; /*0x10316c210*/
            if ( *(_DWORD *)(v112 + 256) != 0 ) /*0x10316c21c*/
            {
              v58 = 0; /*0x10316c222*/
              do /*0x10316c28f*/
              {
                v59 = *(int *)(v57 + 4 * v58); /*0x10316c224*/
                v60 = v59; /*0x10316c228*/
                do /*0x10316c25d*/
                {
                  LOBYTE(v117[0]) = v59 & 0x7F | ((v59 > 0x7F) << 7); /*0x10316c23f*/
                  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c24d*/
                  v60 >>= 7; /*0x10316c252*/
                  v15 = v59 <= 0x7F; /*0x10316c256*/
                  v59 = v60; /*0x10316c25a*/
                }
                while ( !v15 ); /*0x10316c25d*/
                v57 = v112; /*0x10316c25f*/
                v117[0] = *(float *)(v112 + 4 * v58 + 128); /*0x10316c26b*/
                std::string::append(a1: a2, a2: v117, a3: 4); /*0x10316c279*/
                ++v58; /*0x10316c27e*/
                v42 = v107; /*0x10316c28b*/
              }
              while ( v58 < *(unsigned int *)(v112 + 256) ); /*0x10316c28f*/
            }
          }
          else
          {
            LOBYTE(v117[0]) = 5; /*0x10316c339*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c348*/
            v64 = *(unsigned int *)(v54 + 256); /*0x10316c34d*/
            v65 = v64; /*0x10316c353*/
            do /*0x10316c387*/
            {
              LOBYTE(v117[0]) = v64 & 0x7F | ((v64 > 0x7F) << 7); /*0x10316c369*/
              std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c377*/
              v65 >>= 7; /*0x10316c37c*/
              v15 = v64 <= 0x7F; /*0x10316c380*/
              v64 = v65; /*0x10316c384*/
            }
            while ( !v15 ); /*0x10316c387*/
            v66 = v112; /*0x10316c389*/
            if ( *(_DWORD *)(v112 + 256) != 0 ) /*0x10316c394*/
            {
              v67 = 0; /*0x10316c396*/
              do /*0x10316c3e7*/
              {
                v68 = *(int *)(v66 + 4 * v67); /*0x10316c398*/
                v69 = v68; /*0x10316c39c*/
                do /*0x10316c3d1*/
                {
                  LOBYTE(v117[0]) = v68 & 0x7F | ((v68 > 0x7F) << 7); /*0x10316c3b3*/
                  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c3c1*/
                  v69 >>= 7; /*0x10316c3c6*/
                  v15 = v68 <= 0x7F; /*0x10316c3ca*/
                  v68 = v69; /*0x10316c3ce*/
                }
                while ( !v15 ); /*0x10316c3d1*/
                ++v67; /*0x10316c3d3*/
                v66 = v112; /*0x10316c3d6*/
                v42 = v107; /*0x10316c3e3*/
              }
              while ( v67 < *(unsigned int *)(v112 + 256) ); /*0x10316c3e7*/
            }
          }
          goto LABEL_79; /*0x10316c28f*/
        case 9: /*0x10316bec0*/
          LOBYTE(v117[0]) = 6; /*0x10316bf3f*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bf4e*/
          v44 = *(unsigned int *)(v42 + 8); /*0x10316bf53*/
          v45 = v44; /*0x10316bf58*/
          do /*0x10316bf8f*/
          {
            LOBYTE(v117[0]) = v44 & 0x7F | ((v44 > 0x7F) << 7); /*0x10316bf6e*/
            std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316bf7c*/
            v45 >>= 7; /*0x10316bf81*/
            v46 = v44 < 0x80; /*0x10316bf85*/
            v44 = v45; /*0x10316bf8c*/
          }
          while ( !v46 ); /*0x10316bf8f*/
          goto LABEL_79; /*0x10316bf8f*/
        case 0xA: /*0x10316bec0*/
          LOBYTE(v117[0]) = 10; /*0x10316c156*/
          std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c165*/
          Luau::BytecodeBuilder::writeClassShape( /*0x10316c181*/
            a1: v53,
            a2,
            a3: *(_QWORD *)(a1 + 192) + 56LL * *(unsigned int *)(v42 + 8));
          goto LABEL_79; /*0x10316c186*/
        default:
          goto LABEL_79;
      }
      break;
    }
  }
  v72 = (unsigned int)((*(_QWORD *)(a1 + 128) - *(_QWORD *)(a1 + 120)) >> 2); /*0x10316c447*/
  v73 = v72; /*0x10316c460*/
  do /*0x10316c494*/
  {
    LOBYTE(v117[0]) = v72 & 0x7F | ((v72 > 0x7F) << 7); /*0x10316c476*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c484*/
    v73 >>= 7; /*0x10316c489*/
    v15 = v72 <= 0x7F; /*0x10316c48d*/
    v72 = v73; /*0x10316c491*/
  }
  while ( !v15 ); /*0x10316c494*/
  v74 = *(unsigned int **)(a1 + 120); /*0x10316c496*/
  for ( k = *(unsigned int **)(a1 + 128); v74 != k; ++v74 ) /*0x10316c4a8*/
  {
    v75 = *v74; /*0x10316c4ae*/
    v76 = v75; /*0x10316c4b1*/
    do /*0x10316c4e6*/
    {
      LOBYTE(v117[0]) = v75 & 0x7F | ((v75 > 0x7F) << 7); /*0x10316c4c8*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c4d6*/
      v76 >>= 7; /*0x10316c4db*/
      v15 = v75 <= 0x7F; /*0x10316c4df*/
      v75 = v76; /*0x10316c4e3*/
    }
    while ( !v15 ); /*0x10316c4e6*/
  }
  v77 = *(int *)(v109 + 32); /*0x10316c4f6*/
  v78 = v77; /*0x10316c4fe*/
  do /*0x10316c532*/
  {
    LOBYTE(v117[0]) = v77 & 0x7F | ((v77 > 0x7F) << 7); /*0x10316c514*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c522*/
    v78 >>= 7; /*0x10316c527*/
    v15 = v77 <= 0x7F; /*0x10316c52b*/
    v77 = v78; /*0x10316c52f*/
  }
  while ( !v15 ); /*0x10316c532*/
  v79 = *(unsigned int *)(v109 + 28); /*0x10316c538*/
  v80 = v79; /*0x10316c53f*/
  do /*0x10316c577*/
  {
    LOBYTE(v117[0]) = v79 & 0x7F | ((v79 > 0x7F) << 7); /*0x10316c559*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c567*/
    v80 >>= 7; /*0x10316c56c*/
    v15 = v79 <= 0x7F; /*0x10316c570*/
    v79 = v80; /*0x10316c574*/
  }
  while ( !v15 ); /*0x10316c577*/
  for ( m = *(_DWORD **)(a1 + 72); ; ++m ) /*0x10316c579*/
  {
    if ( m == *(_DWORD **)(a1 + 80) ) /*0x10316c584*/
    {
      LOBYTE(v117[0]) = 1; /*0x10316c595*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c5a0*/
      Luau::BytecodeBuilder::writeLineInfo(this: (Luau::BytecodeBuilder *)a1); /*0x10316c5ab*/
      goto LABEL_108; /*0x10316c5b0*/
    }
    if ( *m == 0 ) /*0x10316c589*/
      break; /*0x10316c589*/
  }
  LOBYTE(v117[0]) = 0; /*0x10316c5b6*/
  std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c5c1*/
LABEL_108:
  if ( *(_QWORD *)(a1 + 424) == *(_QWORD *)(a1 + 432) && *(_QWORD *)(a1 + 448) == *(_QWORD *)(a1 + 456) ) /*0x10316c5e4*/
  {
    LOBYTE(v117[0]) = 0; /*0x10316c7f5*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c800*/
  }
  else
  {
    LOBYTE(v117[0]) = 1; /*0x10316c5ee*/
    std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c5fd*/
    v82 = (unsigned int)((*(_QWORD *)(a1 + 432) - *(_QWORD *)(a1 + 424)) >> 4); /*0x10316c614*/
    v83 = v82; /*0x10316c616*/
    do /*0x10316c64a*/
    {
      LOBYTE(v117[0]) = v82 & 0x7F | ((v82 > 0x7F) << 7); /*0x10316c62c*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c63a*/
      v83 >>= 7; /*0x10316c63f*/
      v15 = v82 <= 0x7F; /*0x10316c643*/
      v82 = v83; /*0x10316c647*/
    }
    while ( !v15 ); /*0x10316c64a*/
    v84 = *(unsigned int **)(a1 + 424); /*0x10316c64c*/
    for ( n = *(unsigned int **)(a1 + 432); v84 != n; v84 += 4 ) /*0x10316c661*/
    {
      v85 = *v84; /*0x10316c66b*/
      v86 = v85; /*0x10316c66e*/
      do /*0x10316c6a3*/
      {
        LOBYTE(v117[0]) = v85 & 0x7F | ((v85 > 0x7F) << 7); /*0x10316c685*/
        std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c693*/
        v86 >>= 7; /*0x10316c698*/
        v15 = v85 <= 0x7F; /*0x10316c69c*/
        v85 = v86; /*0x10316c6a0*/
      }
      while ( !v15 ); /*0x10316c6a3*/
      v87 = v84[2]; /*0x10316c6a5*/
      v88 = v87; /*0x10316c6a9*/
      do /*0x10316c6dd*/
      {
        LOBYTE(v117[0]) = v87 & 0x7F | ((v87 > 0x7F) << 7); /*0x10316c6bf*/
        std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c6cd*/
        v88 >>= 7; /*0x10316c6d2*/
        v15 = v87 <= 0x7F; /*0x10316c6d6*/
        v87 = v88; /*0x10316c6da*/
      }
      while ( !v15 ); /*0x10316c6dd*/
      v89 = v84[3]; /*0x10316c6df*/
      v90 = v89; /*0x10316c6e3*/
      do /*0x10316c717*/
      {
        LOBYTE(v117[0]) = v89 & 0x7F | ((v89 > 0x7F) << 7); /*0x10316c6f9*/
        std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c707*/
        v90 >>= 7; /*0x10316c70c*/
        v15 = v89 <= 0x7F; /*0x10316c710*/
        v89 = v90; /*0x10316c714*/
      }
      while ( !v15 ); /*0x10316c717*/
      LOBYTE(v117[0]) = *((_BYTE *)v84 + 4); /*0x10316c71d*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c72b*/
    }
    v91 = (unsigned int)((*(_QWORD *)(a1 + 456) - *(_QWORD *)(a1 + 448)) >> 2); /*0x10316c754*/
    v92 = v91; /*0x10316c75a*/
    do /*0x10316c78e*/
    {
      LOBYTE(v117[0]) = v91 & 0x7F | ((v91 > 0x7F) << 7); /*0x10316c770*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c77e*/
      v92 >>= 7; /*0x10316c783*/
      v15 = v91 <= 0x7F; /*0x10316c787*/
      v91 = v92; /*0x10316c78b*/
    }
    while ( !v15 ); /*0x10316c78e*/
    v93 = *(unsigned int **)(a1 + 448); /*0x10316c790*/
    for ( ii = *(unsigned int **)(a1 + 456); v93 != ii; ++v93 ) /*0x10316c7a5*/
    {
      v94 = *v93; /*0x10316c7ab*/
      v95 = v94; /*0x10316c7ae*/
      do /*0x10316c7e3*/
      {
        LOBYTE(v117[0]) = v94 & 0x7F | ((v94 > 0x7F) << 7); /*0x10316c7c5*/
        std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c7d3*/
        v95 >>= 7; /*0x10316c7d8*/
        v15 = v94 <= 0x7F; /*0x10316c7dc*/
        v94 = v95; /*0x10316c7e0*/
      }
      while ( !v15 ); /*0x10316c7e3*/
    }
  }
  if ( (_BYTE)FFlag::LuauEmitCallFeedback == 1 ) /*0x10316c80c*/
  {
    v96 = (__int64)(*(_QWORD *)(a1 + 224) - *(_QWORD *)(a1 + 216)) >> 2; /*0x10316c824*/
    v97 = v96; /*0x10316c82c*/
    do /*0x10316c860*/
    {
      LOBYTE(v117[0]) = v96 & 0x7F | ((v96 > 0x7F) << 7); /*0x10316c842*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c850*/
      v97 >>= 7; /*0x10316c855*/
      v15 = v96 <= 0x7F; /*0x10316c859*/
      v96 = v97; /*0x10316c85d*/
    }
    while ( !v15 ); /*0x10316c860*/
    v98 = *(__int16 **)(a1 + 216); /*0x10316c862*/
    result = *(__int16 **)(a1 + 224); /*0x10316c869*/
    for ( jj = result; v98 != jj; v98 += 2 ) /*0x10316c877*/
    {
      v100 = *(unsigned int *)v98; /*0x10316c881*/
      LOBYTE(v117[0]) = 0; /*0x10316c884*/
      std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c893*/
      v101 = v100; /*0x10316c898*/
      do /*0x10316c8cd*/
      {
        LOBYTE(v117[0]) = v100 & 0x7F | ((v100 > 0x7F) << 7); /*0x10316c8af*/
        result = (__int16 *)std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c8bd*/
        v101 >>= 7; /*0x10316c8c2*/
        v15 = v100 <= 0x7F; /*0x10316c8c6*/
        v100 = v101; /*0x10316c8ca*/
      }
      while ( !v15 ); /*0x10316c8cd*/
    }
  }
  else if ( (_BYTE)FFlag::LuauBytecodeCostModel != 0 /*0x10316c900*/
         || (_BYTE)FFlag::LuauCompileEmitVectorDouble != 0
         || (_BYTE)FFlag::LuauCompileFastpcall != 0
         || (result = &FFlag::DebugLuauUserDefinedClasses, (_BYTE)FFlag::DebugLuauUserDefinedClasses == 1) )
  {
    LOBYTE(v117[0]) = 0; /*0x10316c906*/
    result = (__int16 *)std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c911*/
  }
  if ( (_BYTE)FFlag::LuauBytecodeCostModel != 0 /*0x10316c92f*/
    || (_BYTE)FFlag::LuauCompileEmitVectorDouble != 0
    || (_BYTE)FFlag::LuauCompileFastpcall != 0 )
  {
    v102 = a5; /*0x10316c935*/
    if ( (v108 & 8) == 0 ) /*0x10316c939*/
      return result; /*0x10316c939*/
    goto LABEL_145; /*0x10316c939*/
  }
  v102 = a5; /*0x10316c989*/
  if ( (v108 & 8) != 0 ) /*0x10316c98d*/
  {
    result = &FFlag::DebugLuauUserDefinedClasses; /*0x10316c98f*/
    if ( (_BYTE)FFlag::DebugLuauUserDefinedClasses != 0 ) /*0x10316c999*/
    {
LABEL_145:
      v103 = v102; /*0x10316c93b*/
      do /*0x10316c974*/
      {
        LOBYTE(v117[0]) = v102 & 0x7F | ((v102 > 0x7F) << 7); /*0x10316c956*/
        result = (__int16 *)std::string::append(a1: a2, a2: v117, a3: 1); /*0x10316c964*/
        v103 >>= 7; /*0x10316c969*/
        v15 = v102 <= 0x7F; /*0x10316c96d*/
        v102 = v103; /*0x10316c971*/
      }
      while ( !v15 ); /*0x10316c974*/
    }
  }
  return result; /*0x10316c976*/
}

// Address: 0x10316c9dc | Name: __ZN4Luau15BytecodeBuilder15setMainFunctionEj
void __fastcall Luau::BytecodeBuilder::setMainFunction(Luau::BytecodeBuilder *this, int a2)
{
  *((_DWORD *)this + 9) = a2; /*0x10316c9e0*/
}

// Address: 0x10316c9e6 | Name: __ZN4Luau15BytecodeBuilder11addConstantERKNS0_11ConstantKeyERKNS0_8ConstantE
__int64 __fastcall Luau::BytecodeBuilder::addConstant(__int64 a1, __int64 a2, __int64 a3)
{
  __int64 v6; // r12
  __int64 v7; // rax
  __int64 result; // rax
  unsigned int v9; // r13d

  v6 = a1 + 248; /*0x10316c9fd*/
  v7 = Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::ConstantKey,std::pair<Luau::BytecodeBuilder::ConstantKey,int>,std::pair<Luau::BytecodeBuilder::ConstantKey const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::ConstantKey,int>,Luau::BytecodeBuilder::ConstantKeyHash,std::equal_to<Luau::BytecodeBuilder::ConstantKey>>::find(a1: a1 + 248); /*0x10316ca07*/
  if ( v7 != 0 ) /*0x10316ca0f*/
    return *(unsigned int *)(v7 + 40); /*0x10316ca11*/
  v9 = -858993459 * ((*(_QWORD *)(a1 + 104) - *(_QWORD *)(a1 + 96)) >> 3); /*0x10316ca22*/
  result = 0xFFFFFFFFLL; /*0x10316ca29*/
  if ( v9 <= 0x7FFFFF ) /*0x10316ca35*/
  {
    Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::ConstantKey,std::pair<Luau::BytecodeBuilder::ConstantKey,int>,std::pair<Luau::BytecodeBuilder::ConstantKey const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::ConstantKey,int>,Luau::BytecodeBuilder::ConstantKeyHash,std::equal_to<Luau::BytecodeBuilder::ConstantKey>>::rehash_if_full( /*0x10316ca41*/
      a1: v6,
      a2);
    *(_DWORD *)(Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::ConstantKey,std::pair<Luau::BytecodeBuilder::ConstantKey,int>,std::pair<Luau::BytecodeBuilder::ConstantKey const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::ConstantKey,int>,Luau::BytecodeBuilder::ConstantKeyHash,std::equal_to<Luau::BytecodeBuilder::ConstantKey>>::insert_unsafe( /*0x10316ca51*/
                  a1: v6,
                  a2)
              + 40) = v9;
    std::vector<Luau::BytecodeBuilder::Constant>::push_back[abi:ne200100](a1: a1 + 96, a2: a3); /*0x10316ca5b*/
    return v9; /*0x10316ca60*/
  }
  return result; /*0x10316ca67*/
}

// Address: 0x10316cb9c | Name: __ZN4Luau15BytecodeBuilder19addStringTableEntryENS0_9StringRefE
__int64 __fastcall Luau::BytecodeBuilder::addStringTableEntry(__int64 a1, __int64 a2, __int64 a3)
{
  __int64 v4; // r15
  __int64 inserted; // r14
  __int64 result; // rax
  _QWORD v7[5]; // [rsp+8h] [rbp-28h] BYREF

  v7[0] = a2; /*0x10316cbb0*/
  v7[1] = a3; /*0x10316cbb3*/
  v4 = a1 + 544; /*0x10316cbb7*/
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::StringRef,std::pair<Luau::BytecodeBuilder::StringRef,unsigned int>,std::pair<Luau::BytecodeBuilder::StringRef const,unsigned int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::StringRef,unsigned int>,Luau::BytecodeBuilder::StringRefHash,std::equal_to<Luau::BytecodeBuilder::StringRef>>::rehash_if_full( /*0x10316cbc4*/
    a1: a1 + 544,
    a2: v7);
  inserted = Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::StringRef,std::pair<Luau::BytecodeBuilder::StringRef,unsigned int>,std::pair<Luau::BytecodeBuilder::StringRef const,unsigned int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::StringRef,unsigned int>,Luau::BytecodeBuilder::StringRefHash,std::equal_to<Luau::BytecodeBuilder::StringRef>>::insert_unsafe( /*0x10316cbd4*/
               a1: v4,
               a2: v7);
  result = *(unsigned int *)(inserted + 16); /*0x10316cbd7*/
  if ( (_DWORD)result == 0 ) /*0x10316cbdc*/
  {
    result = *(unsigned int *)(a1 + 584); /*0x10316cbde*/
    *(_DWORD *)(inserted + 16) = result; /*0x10316cbe4*/
    if ( (*(_BYTE *)(a1 + 704) & 1) != 0 ) /*0x10316cbef*/
    {
      std::vector<Luau::BytecodeBuilder::StringRef>::push_back[abi:ne200100](a1: a1 + 600, a2: v7); /*0x10316cbff*/
      return *(unsigned int *)(inserted + 16); /*0x10316cc04*/
    }
  }
  return result; /*0x10316cc08*/
}

// Address: 0x10316cd0a | Name: __ZNK4Luau15BytecodeBuilder22tryGetUserdataTypeNameE16LuauBytecodeType
__int64 __fastcall Luau::BytecodeBuilder::tryGetUserdataTypeName(__int64 a1, int a2)
{
  unsigned __int64 v2; // rsi
  __int64 v3; // rcx
  __int64 v4; // rax

  v2 = (a2 & 0xFFFFFF7F) - 64; /*0x10316cd10*/
  v3 = *(_QWORD *)(a1 + 520); /*0x10316cd13*/
  if ( (*(_QWORD *)(a1 + 528) - v3) >> 5 <= v2 ) /*0x10316cd2b*/
    return 0; /*0x10316cd44*/
  v4 = v3 + 32 * v2; /*0x10316cd35*/
  if ( (*(_BYTE *)v4 & 1) != 0 ) /*0x10316cd3d*/
    return *(_QWORD *)(v4 + 16); /*0x10316cd47*/
  else
    return v4 + 1; /*0x10316cd3f*/
}

// Address: 0x10316cd4e | Name: __ZN4Luau15BytecodeBuilder14addConstantNilEv
__int64 __fastcall Luau::BytecodeBuilder::addConstantNil(Luau::BytecodeBuilder *this)
{
  _DWORD v2[10]; // [rsp+8h] [rbp-58h] BYREF
  _BYTE v3[48]; // [rsp+30h] [rbp-30h] BYREF

  memset(v3, 0, 40); /*0x10316cd61*/
  v2[0] = 0; /*0x10316cd70*/
  memset(&v2[2], 0, 32); /*0x10316cd76*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v2, a3: (__int64)v3); /*0x10316cd83*/
}

// Address: 0x10316cd8a | Name: __ZN4Luau15BytecodeBuilder18addConstantBooleanEb
__int64 __fastcall Luau::BytecodeBuilder::addConstantBoolean(Luau::BytecodeBuilder *this, unsigned int a2)
{
  _QWORD v3[5]; // [rsp+8h] [rbp-58h] BYREF
  _DWORD v4[12]; // [rsp+30h] [rbp-30h] BYREF

  memset(v4, 0, 40); /*0x10316cd99*/
  v4[0] = 1; /*0x10316cdac*/
  LOBYTE(v4[2]) = a2; /*0x10316cdaf*/
  LODWORD(v3[0]) = 1; /*0x10316cdb7*/
  v3[1] = a2; /*0x10316cdbc*/
  memset(&v3[2], 0, 24); /*0x10316cdc0*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v3, a3: (__int64)v4); /*0x10316cdd0*/
}

// Address: 0x10316cdd6 | Name: __ZN4Luau15BytecodeBuilder17addConstantNumberEd
__int64 __fastcall Luau::BytecodeBuilder::addConstantNumber(Luau::BytecodeBuilder *this, double a2)
{
  _QWORD v3[5]; // [rsp+8h] [rbp-58h] BYREF
  _QWORD v4[6]; // [rsp+30h] [rbp-30h] BYREF

  HIDWORD(v4[0]) = 0; /*0x10316cde5*/
  memset(&v4[2], 0, 24); /*0x10316cde8*/
  LODWORD(v4[0]) = 2; /*0x10316cdf7*/
  *(double *)&v4[1] = a2; /*0x10316cdf9*/
  LODWORD(v3[0]) = 2; /*0x10316ce02*/
  memset(&v3[2], 0, 24); /*0x10316ce04*/
  *(double *)&v3[1] = a2; /*0x10316ce0c*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v3, a3: (__int64)v4); /*0x10316ce16*/
}

// Address: 0x10316ce1c | Name: __ZN4Luau15BytecodeBuilder18addConstantIntegerEx
__int64 __fastcall Luau::BytecodeBuilder::addConstantInteger(Luau::BytecodeBuilder *this, __int64 a2)
{
  _QWORD v3[5]; // [rsp+8h] [rbp-58h] BYREF
  _QWORD v4[6]; // [rsp+30h] [rbp-30h] BYREF

  HIDWORD(v4[0]) = 0; /*0x10316ce2b*/
  memset(&v4[2], 0, 24); /*0x10316ce2e*/
  LODWORD(v4[0]) = 3; /*0x10316ce3e*/
  v4[1] = a2; /*0x10316ce41*/
  LODWORD(v3[0]) = 3; /*0x10316ce49*/
  memset(&v3[2], 0, 24); /*0x10316ce4c*/
  v3[1] = a2; /*0x10316ce54*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v3, a3: (__int64)v4); /*0x10316ce60*/
}

// Address: 0x10316ce66 | Name: __ZN4Luau15BytecodeBuilder18addConstantVectorfEffff
__int64 __fastcall Luau::BytecodeBuilder::addConstantVectorf(
        Luau::BytecodeBuilder *this,
        float a2,
        float a3,
        float a4,
        float a5)
{
  int v6; // [rsp+8h] [rbp-58h] BYREF
  unsigned __int64 v7; // [rsp+10h] [rbp-50h]
  unsigned __int64 v8; // [rsp+18h] [rbp-48h]
  __int128 v9; // [rsp+20h] [rbp-40h]
  _QWORD v10[2]; // [rsp+30h] [rbp-30h] BYREF
  __int128 v11; // [rsp+40h] [rbp-20h]
  __int64 v12; // [rsp+50h] [rbp-10h]

  HIDWORD(v10[0]) = 0; /*0x10316ce75*/
  v11 = 0; /*0x10316ce78*/
  v12 = 0; /*0x10316ce7c*/
  LODWORD(v10[0]) = 4; /*0x10316ce89*/
  v10[1] = __PAIR64__(LODWORD(a3), LODWORD(a2)); /*0x10316ce8b*/
  *(_QWORD *)&v11 = __PAIR64__(LODWORD(a5), LODWORD(a4)); /*0x10316ce95*/
  v6 = 4; /*0x10316cea3*/
  v9 = 0; /*0x10316cea5*/
  v7 = __PAIR64__(LODWORD(a3), LODWORD(a2)); /*0x10316cea9*/
  v8 = __PAIR64__(LODWORD(a5), LODWORD(a4)); /*0x10316ceb3*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)&v6, a3: (__int64)v10); /*0x10316cec2*/
}

// Address: 0x10316cec8 | Name: __ZN4Luau15BytecodeBuilder18addConstantVectordEdddd
__int64 __fastcall Luau::BytecodeBuilder::addConstantVectord(
        Luau::BytecodeBuilder *this,
        double a2,
        double a3,
        double a4,
        double a5)
{
  double v6[5]; // [rsp+0h] [rbp-50h] BYREF
  _QWORD v7[5]; // [rsp+28h] [rbp-28h] BYREF

  *(_QWORD *)&v6[0] = 5; /*0x10316ced4*/
  v6[1] = a2; /*0x10316cee2*/
  v6[2] = a3; /*0x10316cee7*/
  v6[3] = a4; /*0x10316ceec*/
  v6[4] = a5; /*0x10316cef1*/
  LODWORD(v7[0]) = 5; /*0x10316cefa*/
  *(double *)&v7[1] = a2; /*0x10316cefc*/
  *(double *)&v7[2] = a3; /*0x10316cf01*/
  *(double *)&v7[3] = a4; /*0x10316cf06*/
  *(double *)&v7[4] = a5; /*0x10316cf0b*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v7, a3: (__int64)v6); /*0x10316cf15*/
}

// Address: 0x10316cf1c | Name: __ZN4Luau15BytecodeBuilder17addConstantStringENS0_9StringRefE
__int64 __fastcall Luau::BytecodeBuilder::addConstantString(__int64 a1, __int64 a2, __int64 a3)
{
  _QWORD v5[5]; // [rsp+8h] [rbp-58h] BYREF
  _QWORD v6[6]; // [rsp+30h] [rbp-30h] BYREF

  memset(v6, 0, 40); /*0x10316cf34*/
  LODWORD(v6[0]) = 6; /*0x10316cf46*/
  LODWORD(v6[1]) = Luau::BytecodeBuilder::addStringTableEntry(a1, a2, a3); /*0x10316cf48*/
  LODWORD(v5[0]) = 6; /*0x10316cf4f*/
  v5[1] = LODWORD(v6[1]); /*0x10316cf53*/
  memset(&v5[2], 0, 24); /*0x10316cf57*/
  return Luau::BytecodeBuilder::addConstant(a1, a2: (__int64)v5, a3: (__int64)v6); /*0x10316cf67*/
}

// Address: 0x10316cf6e | Name: __ZN4Luau15BytecodeBuilder9addImportEj
__int64 __fastcall Luau::BytecodeBuilder::addImport(Luau::BytecodeBuilder *this, unsigned int a2)
{
  _QWORD v3[5]; // [rsp+8h] [rbp-58h] BYREF
  _DWORD v4[12]; // [rsp+30h] [rbp-30h] BYREF

  memset(v4, 0, 40); /*0x10316cf7d*/
  v4[0] = 7; /*0x10316cf90*/
  v4[2] = a2; /*0x10316cf93*/
  LODWORD(v3[0]) = 7; /*0x10316cf9a*/
  v3[1] = a2; /*0x10316cf9f*/
  memset(&v3[2], 0, 24); /*0x10316cfa3*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v3, a3: (__int64)v4); /*0x10316cfb3*/
}

// Address: 0x10316cfba | Name: __ZN4Luau15BytecodeBuilder16addConstantTableERKNS0_10TableShapeE
__int64 __fastcall Luau::BytecodeBuilder::addConstantTable(_QWORD *a1, __int64 a2)
{
  _QWORD *v4; // r14
  __int64 v5; // rax
  __int64 v6; // rdx
  __int64 v7; // rcx
  __int64 v8; // r8
  __int64 v9; // r9
  __int64 result; // rax
  unsigned int v11; // r12d
  _QWORD v12[6]; // [rsp+0h] [rbp-60h] BYREF
  _QWORD *v13; // [rsp+30h] [rbp-30h]

  v4 = a1 + 38; /*0x10316cfd1*/
  v5 = Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::TableShape,std::pair<Luau::BytecodeBuilder::TableShape,int>,std::pair<Luau::BytecodeBuilder::TableShape const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::TableShape,int>,Luau::BytecodeBuilder::TableShapeHash,std::equal_to<Luau::BytecodeBuilder::TableShape>>::find(a1: a1 + 38); /*0x10316cfdb*/
  if ( v5 != 0 ) /*0x10316cfe3*/
    return *(unsigned int *)(v5 + 264); /*0x10316cfe5*/
  v11 = -858993459 * ((a1[13] - a1[12]) >> 3); /*0x10316cffc*/
  result = 0xFFFFFFFFLL; /*0x10316d003*/
  if ( v11 <= 0x7FFFFF ) /*0x10316d00f*/
  {
    v13 = a1 + 12; /*0x10316d019*/
    memset(v12, 0, 40); /*0x10316d024*/
    LODWORD(v12[0]) = 8; /*0x10316d036*/
    LODWORD(v12[1]) = 1041204193 * ((a1[22] - a1[21]) >> 3); /*0x10316d05d*/
    Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::TableShape,std::pair<Luau::BytecodeBuilder::TableShape,int>,std::pair<Luau::BytecodeBuilder::TableShape const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::TableShape,int>,Luau::BytecodeBuilder::TableShapeHash,std::equal_to<Luau::BytecodeBuilder::TableShape>>::rehash_if_full( /*0x10316d067*/
      a1: v4,
      a2,
      a3: v6,
      a4: v7,
      a5: v8,
      a6: v9,
      a7: v12[0],
      a8: v12[1],
      a9: 0,
      a10: 0,
      a11: 0);
    *(_DWORD *)(Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::TableShape,std::pair<Luau::BytecodeBuilder::TableShape,int>,std::pair<Luau::BytecodeBuilder::TableShape const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::TableShape,int>,Luau::BytecodeBuilder::TableShapeHash,std::equal_to<Luau::BytecodeBuilder::TableShape>>::insert_unsafe( /*0x10316d077*/
                  a1: v4,
                  a2)
              + 264) = v11;
    std::vector<Luau::BytecodeBuilder::TableShape>::push_back[abi:ne200100](a1: a1 + 21, a2); /*0x10316d084*/
    std::vector<Luau::BytecodeBuilder::Constant>::push_back[abi:ne200100](a1: v13, a2: v12); /*0x10316d090*/
    return v11; /*0x10316d095*/
  }
  return result; /*0x10316d098*/
}

// Address: 0x10316d1d6 | Name: __ZN4Luau15BytecodeBuilder18addConstantClosureEj
__int64 __fastcall Luau::BytecodeBuilder::addConstantClosure(Luau::BytecodeBuilder *this, unsigned int a2)
{
  _QWORD v3[5]; // [rsp+8h] [rbp-58h] BYREF
  _DWORD v4[12]; // [rsp+30h] [rbp-30h] BYREF

  memset(v4, 0, 40); /*0x10316d1e5*/
  v4[0] = 9; /*0x10316d1f8*/
  v4[2] = a2; /*0x10316d1fb*/
  LODWORD(v3[0]) = 9; /*0x10316d202*/
  v3[1] = a2; /*0x10316d207*/
  memset(&v3[2], 0, 24); /*0x10316d20b*/
  return Luau::BytecodeBuilder::addConstant(a1: (__int64)this, a2: (__int64)v3, a3: (__int64)v4); /*0x10316d21b*/
}

// Address: 0x10316d222 | Name: __ZN4Luau15BytecodeBuilder9addFbSlotE16LuauFeedbackType
__int64 __fastcall Luau::BytecodeBuilder::addFbSlot(_QWORD *a1)
{
  __int64 v1; // rax
  _DWORD v4[3]; // [rsp+0h] [rbp-Ch] BYREF

  v4[0] = HIDWORD(v1); /*0x10316d227*/
  v4[0] = (a1[7] - a1[6]) >> 2; /*0x10316d242*/
  std::vector<unsigned int>::push_back[abi:ne200100](a1: a1 + 27, a2: v4); /*0x10316d244*/
  return (unsigned int)((a1[28] - a1[27]) >> 2) - 1; /*0x10316d261*/
}

// Address: 0x10316d264 | Name: __ZNK4Luau15BytecodeBuilder19getInstructionCountEv
__int64 __fastcall Luau::BytecodeBuilder::getInstructionCount(Luau::BytecodeBuilder *this)
{
  return (__int64)(*((_QWORD *)this + 7) - *((_QWORD *)this + 6)) >> 2; /*0x10316d274*/
}

// Address: 0x10316d276 | Name: __ZN4Luau15BytecodeBuilder16addChildFunctionEj
int __fastcall Luau::BytecodeBuilder::addChildFunction(Luau::BytecodeBuilder *this, int a2)
{
  char *v3; // r14
  __int64 v4; // rax
  __int64 v5; // r12
  unsigned int v6; // r12d
  __int64 v7; // rdx
  _DWORD v9[9]; // [rsp+Ch] [rbp-24h] BYREF

  v9[0] = a2; /*0x10316d28c*/
  v3 = (char *)this + 360; /*0x10316d28e*/
  v4 = Luau::detail::DenseHashTable2<unsigned int,std::pair<unsigned int,short>,std::pair<unsigned int const,short>,Luau::detail::ItemInterfaceMap2<unsigned int,short>,std::hash<unsigned int>,std::equal_to<unsigned int>>::find( /*0x10316d29b*/
         a1: (char *)this + 360,
         a2: v9);
  if ( v4 != 0 ) /*0x10316d2a3*/
  {
    LOWORD(v4) = *(_WORD *)(v4 + 4); /*0x10316d2a5*/
  }
  else
  {
    v5 = *((_QWORD *)this + 16) - *((_QWORD *)this + 15); /*0x10316d2b2*/
    LOWORD(v4) = -1; /*0x10316d2b6*/
    if ( (v5 & 0x3FFFE0000LL) == 0 ) /*0x10316d2c7*/
    {
      v6 = (unsigned int)v5 >> 2; /*0x10316d2cd*/
      Luau::detail::DenseHashTable2<unsigned int,std::pair<unsigned int,short>,std::pair<unsigned int const,short>,Luau::detail::ItemInterfaceMap2<unsigned int,short>,std::hash<unsigned int>,std::equal_to<unsigned int>>::rehash_if_full( /*0x10316d2db*/
        a1: v3,
        a2: v9);
      *(_WORD *)(Luau::detail::DenseHashTable2<unsigned int,std::pair<unsigned int,short>,std::pair<unsigned int const,short>,Luau::detail::ItemInterfaceMap2<unsigned int,short>,std::hash<unsigned int>,std::equal_to<unsigned int>>::insert_unsafe( /*0x10316d2eb*/
                   a1: v3,
                   a2: v9)
               + 4) = v6;
      std::vector<unsigned int>::push_back[abi:ne200100](a1: (char *)this + 120, a2: v9, a3: v7); /*0x10316d2f6*/
      LOWORD(v4) = v6; /*0x10316d2fb*/
    }
  }
  LODWORD(v4) = (__int16)v4; /*0x10316d2fe*/
  return v4; /*0x10316d2ff*/
}

// Address: 0x10316d30c | Name: __ZN4Luau15BytecodeBuilder13addClassShapeENS0_10ClassShapeE
__int64 __fastcall Luau::BytecodeBuilder::addClassShape(
        _QWORD *a1,
        __int64 a2,
        __int64 a3,
        __int64 a4,
        __int64 a5,
        __int64 a6)
{
  unsigned int v6; // r15d
  __int64 result; // rax
  _QWORD *v9; // r14
  unsigned __int64 v10; // rax
  __int64 v11; // rcx
  __int64 v12; // rax
  __int128 v13; // [rsp+0h] [rbp-40h] BYREF
  __int128 v14; // [rsp+10h] [rbp-30h]
  __int64 v15; // [rsp+20h] [rbp-20h]

  v6 = -858993459 * ((a1[13] - a1[12]) >> 3); /*0x10316d325*/
  result = 0xFFFFFFFFLL; /*0x10316d32c*/
  if ( v6 <= 0x7FFFFF ) /*0x10316d338*/
  {
    v9 = a1 + 12; /*0x10316d341*/
    v13 = 0; /*0x10316d348*/
    v14 = 0; /*0x10316d34c*/
    v15 = 0; /*0x10316d350*/
    LODWORD(v13) = 10; /*0x10316d358*/
    v10 = a1[25]; /*0x10316d35f*/
    v11 = -1227133513 * (unsigned int)((v10 - a1[24]) >> 3); /*0x10316d374*/
    DWORD2(v13) = -1227133513 * ((v10 - a1[24]) >> 3); /*0x10316d37a*/
    if ( v10 >= a1[26] ) /*0x10316d384*/
    {
      v12 = std::vector<Luau::BytecodeBuilder::ClassShape>::__emplace_back_slow_path<Luau::BytecodeBuilder::ClassShape>( /*0x10316d3d9*/
              a1: a1 + 24,
              a2,
              a3,
              a4: v11,
              a5,
              a6,
              a7: v13,
              a8: *((_QWORD *)&v13 + 1),
              a9: v14,
              a10: *((_QWORD *)&v14 + 1),
              a11: v15);
    }
    else
    {
      *(_DWORD *)v10 = *(_DWORD *)a2; /*0x10316d388*/
      *(_OWORD *)(v10 + 8) = 0; /*0x10316d38a*/
      *(_QWORD *)(v10 + 24) = 0; /*0x10316d390*/
      *(_OWORD *)(v10 + 8) = *(_OWORD *)(a2 + 8); /*0x10316d398*/
      *(_QWORD *)(v10 + 24) = *(_QWORD *)(a2 + 24); /*0x10316d3a0*/
      *(_QWORD *)(a2 + 24) = 0; /*0x10316d3a4*/
      *(_OWORD *)(a2 + 8) = 0; /*0x10316d3a8*/
      *(_QWORD *)(v10 + 48) = 0; /*0x10316d3ac*/
      *(_OWORD *)(v10 + 32) = 0; /*0x10316d3b0*/
      *(_OWORD *)(v10 + 32) = *(_OWORD *)(a2 + 32); /*0x10316d3b8*/
      *(_QWORD *)(v10 + 48) = *(_QWORD *)(a2 + 48); /*0x10316d3c0*/
      *(_QWORD *)(a2 + 48) = 0; /*0x10316d3c4*/
      *(_OWORD *)(a2 + 32) = 0; /*0x10316d3c8*/
      v12 = v10 + 56; /*0x10316d3cc*/
    }
    a1[25] = v12; /*0x10316d3de*/
    std::vector<Luau::BytecodeBuilder::Constant>::push_back[abi:ne200100](a1: v9, a2: &v13); /*0x10316d3ec*/
    return v6; /*0x10316d3f1*/
  }
  return result; /*0x10316d3f4*/
}

// Address: 0x10316d400 | Name: __ZN4Luau15BytecodeBuilder7emitABCE10LuauOpcodehhh
__int64 __fastcall Luau::BytecodeBuilder::emitABC(__int64 a1, int a2, int a3, int a4, int a5)
{
  __int64 v6; // rdx
  _DWORD v8[3]; // [rsp+0h] [rbp-Ch] BYREF

  v6 = a2 | (unsigned int)(a3 << 8); /*0x10316d40c*/
  v8[0] = v6 | (a4 << 16) | (a5 << 24); /*0x10316d41f*/
  std::vector<unsigned int>::push_back[abi:ne200100](a1: a1 + 48, a2: v8, a3: v6); /*0x10316d426*/
  return std::vector<int>::push_back[abi:ne200100](a1: a1 + 72, a2: a1 + 416); /*0x10316d442*/
}

// Address: 0x10316d446 | Name: __ZN4Luau15BytecodeBuilder6emitADE10LuauOpcodehs
__int64 __fastcall Luau::BytecodeBuilder::emitAD(__int64 a1, int a2, int a3, int a4)
{
  __int64 v5; // rdx
  _DWORD v7[3]; // [rsp+0h] [rbp-Ch] BYREF

  v5 = a2 | (unsigned int)(a3 << 8); /*0x10316d452*/
  v7[0] = v5 | (a4 << 16); /*0x10316d45d*/
  std::vector<unsigned int>::push_back[abi:ne200100](a1: a1 + 48, a2: v7, a3: v5); /*0x10316d463*/
  return std::vector<int>::push_back[abi:ne200100](a1: a1 + 72, a2: a1 + 416); /*0x10316d47f*/
}

// Address: 0x10316d482 | Name: __ZN4Luau15BytecodeBuilder5emitEE10LuauOpcodei
__int64 __fastcall Luau::BytecodeBuilder::emitE(__int64 a1, int a2, int a3)
{
  __int64 v4; // rdx
  _DWORD v6[3]; // [rsp+0h] [rbp-Ch] BYREF

  v4 = a2 | (unsigned int)(a3 << 8); /*0x10316d48e*/
  v6[0] = v4; /*0x10316d494*/
  std::vector<unsigned int>::push_back[abi:ne200100](a1: a1 + 48, a2: v6, a3: v4); /*0x10316d49a*/
  return std::vector<int>::push_back[abi:ne200100](a1: a1 + 72, a2: a1 + 416); /*0x10316d4b6*/
}

// Address: 0x10316d4ba | Name: __ZN4Luau15BytecodeBuilder7emitAuxEj
__int64 __fastcall Luau::BytecodeBuilder::emitAux(Luau::BytecodeBuilder *this, int a2, __int64 a3)
{
  _DWORD v5[3]; // [rsp+0h] [rbp-Ch] BYREF

  v5[0] = a2; /*0x10316d4c7*/
  std::vector<unsigned int>::push_back[abi:ne200100](a1: (char *)this + 48, a2: v5, a3); /*0x10316d4d0*/
  return std::vector<int>::push_back[abi:ne200100](a1: (char *)this + 72, a2: (char *)this + 416); /*0x10316d4ec*/
}

// Address: 0x10316d4f0 | Name: __ZN4Luau15BytecodeBuilder8undoEmitE10LuauOpcode
void __fastcall Luau::BytecodeBuilder::undoEmit(__int64 a1)
{
  *(_QWORD *)(a1 + 56) -= 4LL; /*0x10316d4f4*/
  *(_QWORD *)(a1 + 80) -= 4LL; /*0x10316d4f9*/
}

// Address: 0x10316d500 | Name: __ZN4Luau15BytecodeBuilder9emitLabelEv
__int64 __fastcall Luau::BytecodeBuilder::emitLabel(Luau::BytecodeBuilder *this)
{
  return (__int64)(*((_QWORD *)this + 7) - *((_QWORD *)this + 6)) >> 2; /*0x10316d510*/
}

// Address: 0x10316d512 | Name: __ZN4Luau15BytecodeBuilder10patchJumpDEmm
char __fastcall Luau::BytecodeBuilder::patchJumpD(Luau::BytecodeBuilder *this, __int64 a2, int a3)
{
  int v3; // eax
  unsigned int v4; // ecx
  _DWORD v6[4]; // [rsp-10h] [rbp-10h] BYREF

  v3 = a3 + ~(_DWORD)a2; /*0x10316d516*/
  if ( (__int16)(a3 + ~(_WORD)a2) == v3 ) /*0x10316d51d*/
  {
    *(_DWORD *)(*((_QWORD *)this + 6) + 4 * a2) |= v3 << 16; /*0x10316d526*/
LABEL_7:
    v6[0] = a2; /*0x10316d541*/
    v6[1] = a3; /*0x10316d556*/
    std::vector<Luau::BytecodeBuilder::Jump>::push_back[abi:ne200100](a1: (char *)this + 144, a2: v6); /*0x10316d55c*/
    return 1; /*0x10316d568*/
  }
  v4 = -v3; /*0x10316d52d*/
  if ( v3 > 0 ) /*0x10316d52f*/
    v4 = a3 + ~(_DWORD)a2; /*0x10316d52f*/
  if ( v4 <= 0x7FFFFF ) /*0x10316d538*/
  {
    *((_BYTE *)this + 240) = 1; /*0x10316d53a*/
    goto LABEL_7; /*0x10316d53a*/
  }
  return 0; /*0x10316d568*/
}

// Address: 0x10316d654 | Name: __ZN4Luau15BytecodeBuilder10patchSkipCEmm
bool __fastcall Luau::BytecodeBuilder::patchSkipC(Luau::BytecodeBuilder *this, __int64 a2, int a3)
{
  unsigned int v3; // eax

  v3 = a3 + ~(_DWORD)a2; /*0x10316d658*/
  if ( v3 <= 0xFF ) /*0x10316d65f*/
    *(_DWORD *)(*((_QWORD *)this + 6) + 4 * a2) |= v3 << 24; /*0x10316d66e*/
  return v3 < 0x100; /*0x10316d67a*/
}

// Address: 0x10316d67c | Name: __ZN4Luau15BytecodeBuilder8patchAuxEmi
__int64 __fastcall Luau::BytecodeBuilder::patchAux(Luau::BytecodeBuilder *this, __int64 a2, int a3)
{
  __int64 result; // rax

  result = *((_QWORD *)this + 6); /*0x10316d680*/
  *(_DWORD *)(result + 4 * a2) = a3; /*0x10316d684*/
  return result; /*0x10316d687*/
}

// Address: 0x10316d68a | Name: __ZN4Luau15BytecodeBuilder19setFunctionTypeInfoENSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEE
__int64 __fastcall Luau::BytecodeBuilder::setFunctionTypeInfo(__int64 a1, __int64 a2)
{
  __int64 v3; // rcx
  __int64 v4; // rax
  __int64 v5; // r14
  __int64 result; // rax

  v3 = *(_QWORD *)(a1 + 8); /*0x10316d697*/
  v4 = 136LL * *(unsigned int *)(a1 + 32); /*0x10316d6a2*/
  v5 = v3 + v4 + 112; /*0x10316d6aa*/
  if ( (*(_BYTE *)v5 & 1) != 0 ) /*0x10316d6b2*/
    operator delete(a1: *(void **)(v3 + v4 + 128)); /*0x10316d6b8*/
  result = *(_QWORD *)(a2 + 16); /*0x10316d6bd*/
  *(_QWORD *)(v5 + 16) = result; /*0x10316d6c1*/
  *(_OWORD *)v5 = *(_OWORD *)a2; /*0x10316d6c8*/
  *(_WORD *)a2 = 0; /*0x10316d6cc*/
  return result; /*0x10316d6d1*/
}

// Address: 0x10316d6d6 | Name: __ZN4Luau15BytecodeBuilder17pushLocalTypeInfoE16LuauBytecodeTypehjj
__int64 __fastcall Luau::BytecodeBuilder::pushLocalTypeInfo(__int64 a1, int a2, char a3, int a4, int a5)
{
  _DWORD v6[4]; // [rsp+0h] [rbp-10h] BYREF

  v6[0] = a2; /*0x10316d6e2*/
  LOBYTE(v6[1]) = a3; /*0x10316d6e4*/
  v6[2] = a4; /*0x10316d6e7*/
  v6[3] = a5; /*0x10316d6ea*/
  return std::vector<Luau::BytecodeBuilder::TypedLocal>::push_back[abi:ne200100](a1: a1 + 472, a2: v6); /*0x10316d6fd*/
}

// Address: 0x10316d7fa | Name: __ZN4Luau15BytecodeBuilder17pushUpvalTypeInfoE16LuauBytecodeType
__int64 __fastcall Luau::BytecodeBuilder::pushUpvalTypeInfo(__int64 a1, int a2)
{
  int v3; // [rsp+Ch] [rbp-4h] BYREF

  v3 = a2; /*0x10316d806*/
  return std::vector<Luau::BytecodeBuilder::TypedUpval>::push_back[abi:ne200100](a1: a1 + 496, a2: &v3); /*0x10316d817*/
}

// Address: 0x10316d91e | Name: __ZN4Luau15BytecodeBuilder15addUserdataTypeEPKc
__int64 __fastcall Luau::BytecodeBuilder::addUserdataType(Luau::BytecodeBuilder *this, const char *a2)
{
  __int64 v3; // r14
  __int64 v4; // rbx
  void *v6[6]; // [rsp+0h] [rbp-30h] BYREF

  memset(v6, 0, 29); /*0x10316d937*/
  std::string::assign(a1: v6, a2); /*0x10316d93a*/
  std::vector<Luau::BytecodeBuilder::UserdataType>::push_back[abi:ne200100](a1: (char *)this + 520, a2: v6); /*0x10316d94a*/
  v3 = *((_QWORD *)this + 65); /*0x10316d94f*/
  v4 = *((_QWORD *)this + 66); /*0x10316d956*/
  if ( ((__int64)v6[0] & 1) != 0 ) /*0x10316d961*/
    operator delete(a1: v6[2]); /*0x10316d967*/
  return (unsigned int)((unsigned __int64)(v4 - v3) >> 5) - 1; /*0x10316d977*/
}

// Address: 0x10316dae8 | Name: __ZN4Luau15BytecodeBuilder15useUserdataTypeEj
__int64 __fastcall Luau::BytecodeBuilder::useUserdataType(Luau::BytecodeBuilder *this, unsigned int a2)
{
  __int64 result; // rax

  result = 32LL * a2; /*0x10316daf5*/
  *(_BYTE *)(*((_QWORD *)this + 65) + result + 28) = 1; /*0x10316daf9*/
  return result; /*0x10316dafe*/
}

// Address: 0x10316db00 | Name: __ZN4Luau15BytecodeBuilder20setDebugFunctionNameENS0_9StringRefE
__int64 __fastcall Luau::BytecodeBuilder::setDebugFunctionName(__int64 a1, __int64 a2, __int64 a3)
{
  __int64 result; // rax
  __int64 v7; // rcx
  __int64 v8; // rax
  __int64 v9; // rbx
  __int128 v10; // [rsp+0h] [rbp-30h] BYREF
  __int64 v11; // [rsp+10h] [rbp-20h]

  result = Luau::BytecodeBuilder::addStringTableEntry(a1, a2, a3); /*0x10316db16*/
  *(_DWORD *)(*(_QWORD *)(a1 + 8) + 136LL * *(unsigned int *)(a1 + 32) + 28) = result; /*0x10316db2d*/
  if ( *(_QWORD *)(a1 + 784) != 0 ) /*0x10316db39*/
  {
    std::string::basic_string[abi:ne200100](a1: &v10, a2, a3); /*0x10316db45*/
    v7 = *(_QWORD *)(a1 + 8); /*0x10316db4d*/
    v8 = 136LL * *(unsigned int *)(a1 + 32); /*0x10316db58*/
    v9 = v7 + v8 + 64; /*0x10316db60*/
    if ( (*(_BYTE *)v9 & 1) != 0 ) /*0x10316db67*/
      operator delete(a1: *(void **)(v7 + v8 + 80)); /*0x10316db6d*/
    result = v11; /*0x10316db72*/
    *(_QWORD *)(v9 + 16) = v11; /*0x10316db76*/
    *(_OWORD *)v9 = v10; /*0x10316db7e*/
  }
  return result; /*0x10316db81*/
}

// Address: 0x10316db8c | Name: __ZN4Luau15BytecodeBuilder27setDebugFunctionLineDefinedEi
__int64 __fastcall Luau::BytecodeBuilder::setDebugFunctionLineDefined(Luau::BytecodeBuilder *this, int a2)
{
  __int64 result; // rax

  result = 136LL * *((unsigned int *)this + 8); /*0x10316db9e*/
  *(_DWORD *)(*((_QWORD *)this + 1) + result + 32) = a2; /*0x10316dba2*/
  return result; /*0x10316dba6*/
}

// Address: 0x10316dba8 | Name: __ZN4Luau15BytecodeBuilder12setDebugLineEi
void __fastcall Luau::BytecodeBuilder::setDebugLine(Luau::BytecodeBuilder *this, int a2)
{
  *((_DWORD *)this + 104) = a2; /*0x10316dbac*/
}

// Address: 0x10316dbb4 | Name: __ZN4Luau15BytecodeBuilder14pushDebugLocalENS0_9StringRefEhjj
__int64 __fastcall Luau::BytecodeBuilder::pushDebugLocal(__int64 a1, __int64 a2, __int64 a3)
{
  Luau::BytecodeBuilder::addStringTableEntry(a1, a2, a3); /*0x10316dbcf*/
  return std::vector<Luau::BytecodeBuilder::DebugLocal>::push_back[abi:ne200100](a1: a1 + 424); /*0x10316dbf4*/
}

// Address: 0x10316dcf8 | Name: __ZN4Luau15BytecodeBuilder14pushDebugUpvalENS0_9StringRefE
__int64 __fastcall Luau::BytecodeBuilder::pushDebugUpval(__int64 a1, __int64 a2, __int64 a3)
{
  Luau::BytecodeBuilder::addStringTableEntry(a1, a2, a3); /*0x10316dd01*/
  return std::vector<Luau::BytecodeBuilder::DebugUpval>::push_back[abi:ne200100](a1: a1 + 448); /*0x10316dd1f*/
}

// Address: 0x10316de22 | Name: __ZNK4Luau15BytecodeBuilder24getTotalInstructionCountEv
__int64 __fastcall Luau::BytecodeBuilder::getTotalInstructionCount(Luau::BytecodeBuilder *this)
{
  return *((_QWORD *)this + 5); /*0x10316de2a*/
}

// Address: 0x10316de2c | Name: __ZNK4Luau15BytecodeBuilder10getDebugPCEv
__int64 __fastcall Luau::BytecodeBuilder::getDebugPC(Luau::BytecodeBuilder *this)
{
  return (*((_QWORD *)this + 7) - *((_QWORD *)this + 6)) >> 2; /*0x10316de3c*/
}

// Address: 0x10316de3e | Name: __ZN4Luau15BytecodeBuilder14addDebugRemarkEPKcz
__int64 Luau::BytecodeBuilder::addDebugRemark(Luau::BytecodeBuilder *this, const char *a2, ...)
{
  char *v3; // r14
  unsigned int v4; // r15d
  __int64 v5; // r15
  __int64 v6; // r12
  char *v7; // rdx
  unsigned __int64 v8; // rcx
  _DWORD *v9; // rdx
  _DWORD *v10; // r12
  _BYTE *v11; // rsi
  signed __int64 v12; // rdx
  __int64 v13; // r13
  __int64 v14; // rcx
  __int64 v15; // rax
  __int64 v16; // rax
  __int64 v17; // rdx
  __int64 v18; // rcx
  __int64 v19; // rdi
  _DWORD *v20; // r14
  _DWORD *v21; // r14
  void *v22; // rdi
  char *v23; // rsi
  char *v24; // rsi
  _DWORD *v25; // r14
  __int64 v26; // r14
  char *v28; // [rsp+B0h] [rbp-60h] BYREF
  __int64 v29; // [rsp+B8h] [rbp-58h]
  va_list va; // [rsp+C0h] [rbp-50h] BYREF

  if ( (*((_BYTE *)this + 704) & 0x10) != 0 ) /*0x10316deb9*/
  {
    v3 = (char *)this + 648; /*0x10316dec2*/
    v4 = *((unsigned __int8 *)this + 648); /*0x10316dec9*/
    if ( (v4 & 1) != 0 ) /*0x10316ded5*/
      v5 = *((_QWORD *)this + 82); /*0x10316dedc*/
    else
      v5 = v4 >> 1; /*0x10316ded7*/
    va_start(va, a2); /*0x10316def6*/
    Luau::vformatAppend(a1: (char *)this + 648, a2, a3: va); /*0x10316df0a*/
    std::string::push_back(a1: v3, a2: 0); /*0x10316df14*/
    v6 = (*((_QWORD *)this + 7) - *((_QWORD *)this + 6)) >> 2; /*0x10316df21*/
    v7 = *((char **)this + 79); /*0x10316df25*/
    v8 = *((_QWORD *)this + 80); /*0x10316df2c*/
    if ( (unsigned __int64)v7 >= v8 ) /*0x10316df36*/
    {
      v11 = *((_BYTE **)this + 78); /*0x10316df4b*/
      v12 = v7 - v11; /*0x10316df52*/
      v13 = v12 >> 3; /*0x10316df58*/
      if ( (unsigned __int64)((v12 >> 3) + 1) >> 61 != 0 ) /*0x10316df67*/
        std::vector<std::pair<unsigned int,unsigned int>>::__throw_length_error[abi:ne200100](); /*0x10316e0c2*/
      v14 = v8 - (_QWORD)v11; /*0x10316df77*/
      v15 = v14 >> 2; /*0x10316df7d*/
      if ( v14 >> 2 <= (unsigned __int64)((v12 >> 3) + 1) ) /*0x10316df84*/
        v15 = (v12 >> 3) + 1; /*0x10316df84*/
      if ( (unsigned __int64)v14 >= 0x7FFFFFFFFFFFFFF8LL ) /*0x10316df95*/
        v15 = 0x1FFFFFFFFFFFFFFFLL; /*0x10316df95*/
      if ( v15 != 0 ) /*0x10316df9c*/
      {
        v16 = std::__allocate_at_least[abi:ne200100]<std::allocator<std::pair<unsigned int,unsigned int>>>( /*0x10316dfa8*/
                a1: (char *)this + 640,
                a2: v15,
                a3: v12,
                a4: v14,
                a5: 0x1FFFFFFFFFFFFFFFLL);
        v18 = v17; /*0x10316dfad*/
        v11 = *((_BYTE **)this + 78); /*0x10316dfb0*/
        v12 = *((_QWORD *)this + 79) - (_QWORD)v11; /*0x10316dfbe*/
        v19 = v12 >> 3; /*0x10316dfc4*/
      }
      else
      {
        v16 = 0; /*0x10316dfca*/
        v19 = v12 >> 3; /*0x10316dfcc*/
        v18 = 0; /*0x10316dfcf*/
      }
      v20 = (_DWORD *)(v16 + 8 * v13); /*0x10316dfd1*/
      v29 = v16 + 8 * v18; /*0x10316dfd9*/
      *v20 = v6; /*0x10316dfdd*/
      v20[1] = v5; /*0x10316dfe0*/
      v10 = v20 + 2; /*0x10316dfe8*/
      v21 = &v20[-2 * v19]; /*0x10316dff0*/
      memcpy(__dst: v21, __src: v11, __n: v12); /*0x10316dff6*/
      v22 = *((void **)this + 78); /*0x10316dffb*/
      *((_QWORD *)this + 78) = v21; /*0x10316e002*/
      *((_QWORD *)this + 79) = v10; /*0x10316e009*/
      *((_QWORD *)this + 80) = v29; /*0x10316e014*/
      if ( v22 != nullptr ) /*0x10316e01e*/
        operator delete(a1: v22); /*0x10316e020*/
    }
    else
    {
      *(_DWORD *)v7 = v6; /*0x10316df38*/
      *((_DWORD *)v7 + 1) = v5; /*0x10316df3b*/
      v9 = v7 + 8; /*0x10316df3f*/
      v10 = v9; /*0x10316df43*/
    }
    *((_QWORD *)this + 79) = v10; /*0x10316e025*/
    if ( (*((_BYTE *)this + 648) & 1) != 0 ) /*0x10316e033*/
      v23 = *((char **)this + 83); /*0x10316e03e*/
    else
      v23 = (char *)this + 649; /*0x10316e035*/
    v24 = &v23[v5]; /*0x10316e04c*/
    v28 = v24; /*0x10316e04f*/
    v25 = *((_DWORD **)this + 93); /*0x10316e053*/
    if ( (unsigned __int64)v25 >= *((_QWORD *)this + 94) ) /*0x10316e061*/
    {
      v26 = std::vector<std::pair<int,std::string>>::__emplace_back_slow_path<int &,char const*>( /*0x10316e091*/
              a1: (char *)this + 736,
              a2: (char *)this + 416,
              a3: &v28);
    }
    else
    {
      *v25 = *((_DWORD *)this + 104); /*0x10316e065*/
      std::string::basic_string[abi:ne200100]<0>(a1: v25 + 2, a2: v24, a3: v9, a4: v8); /*0x10316e06c*/
      v26 = (__int64)(v25 + 8); /*0x10316e071*/
      *((_QWORD *)this + 93) = v26; /*0x10316e075*/
    }
    *((_QWORD *)this + 93) = v26; /*0x10316e094*/
  }
  return __stack_chk_guard; /*0x10316e0ab*/
}

// Address: 0x10316e0da | Name: __ZN4Luau15BytecodeBuilder8finalizeEv
__int64 __fastcall Luau::BytecodeBuilder::finalize(Luau::BytecodeBuilder *this)
{
  unsigned __int8 *v1; // r14
  unsigned __int8 *v2; // r15
  unsigned int v3; // edx
  __int64 v4; // rsi
  __int64 v5; // rdx
  Luau::BytecodeBuilder *v6; // r15
  unsigned __int64 v7; // rax
  unsigned __int64 v8; // rcx
  __int64 v9; // rsi
  __int64 v10; // rsi
  unsigned __int64 v11; // rdi
  bool v12; // cc
  __int64 v13; // r9
  unsigned __int64 v14; // r8
  unsigned __int8 *i; // rax
  unsigned int v17; // edx
  __int64 v18; // rdx
  char *v19; // r14
  unsigned __int8 v20; // al
  __int64 v21; // rax
  __int64 v22; // rcx
  unsigned __int64 v23; // r13
  unsigned __int64 v24; // rbx
  unsigned __int64 v25; // r15
  unsigned __int64 v26; // r12
  unsigned __int64 v27; // r13
  unsigned __int8 *v28; // r12
  unsigned int v29; // r13d
  unsigned __int64 v30; // r13
  unsigned __int64 v31; // rbx
  unsigned int v32; // edx
  unsigned __int8 *v33; // rsi
  __int64 v34; // rdx
  unsigned __int64 v35; // r15
  unsigned __int64 v36; // r12
  __int64 result; // rax
  unsigned __int8 *j; // [rsp+0h] [rbp-40h]
  char v40; // [rsp+11h] [rbp-2Fh] BYREF
  char v41; // [rsp+12h] [rbp-2Eh] BYREF
  char v42; // [rsp+13h] [rbp-2Dh] BYREF
  char v43; // [rsp+14h] [rbp-2Ch] BYREF
  char v44; // [rsp+15h] [rbp-2Bh] BYREF
  char v45; // [rsp+16h] [rbp-2Ah] BYREF
  _BYTE v46[41]; // [rsp+17h] [rbp-29h] BYREF

  v1 = *((unsigned __int8 **)this + 65); /*0x10316e0eb*/
  v2 = *((unsigned __int8 **)this + 66); /*0x10316e0f6*/
  while ( v1 != v2 ) /*0x10316e100*/
  {
    if ( v1[28] == 1 ) /*0x10316e107*/
    {
      v3 = *v1; /*0x10316e109*/
      if ( (v3 & 1) != 0 ) /*0x10316e110*/
      {
        v5 = *((_QWORD *)v1 + 1); /*0x10316e11a*/
        v4 = *((_QWORD *)v1 + 2); /*0x10316e11e*/
      }
      else
      {
        v4 = (__int64)(v1 + 1); /*0x10316e112*/
        v5 = v3 >> 1; /*0x10316e116*/
      }
      *((_DWORD *)v1 + 6) = Luau::BytecodeBuilder::addStringTableEntry(a1: (__int64)this, a2: v4, a3: v5); /*0x10316e12b*/
    }
    v1 += 32; /*0x10316e12f*/
  }
  v6 = this; /*0x10316e135*/
  v7 = *((_QWORD *)this + 72); /*0x10316e139*/
  if ( v7 != 0 ) /*0x10316e143*/
  {
    v8 = 0; /*0x10316e14c*/
    while ( 1 ) /*0x10316e155*/
    {
      v9 = *(_QWORD *)(*((_QWORD *)this + 71) + 8 * (v8 >> 6)); /*0x10316e155*/
      if ( _bittest64(&v9, v8) ) /*0x10316e15d*/
        break; /*0x10316e15d*/
      if ( v7 == ++v8 ) /*0x10316e165*/
      {
        v10 = 16; /*0x10316e167*/
        goto LABEL_22; /*0x10316e16c*/
      }
    }
  }
  else
  {
    v8 = 0; /*0x10316e16e*/
  }
  v10 = 16; /*0x10316e170*/
  while ( v8 != v7 ) /*0x10316e178*/
  {
    v10 += *(_QWORD *)(*((_QWORD *)this + 68) + 24 * v8 + 8) + 2LL; /*0x10316e18d*/
    v11 = v8 + 1; /*0x10316e194*/
    v12 = v7 <= ++v8; /*0x10316e197*/
    if ( !v12 ) /*0x10316e19d*/
      v8 = *((_QWORD *)this + 72); /*0x10316e19d*/
    while ( v8 != v11 ) /*0x10316e1a4*/
    {
      v13 = *(_QWORD *)(*((_QWORD *)this + 71) + 8 * (v11 >> 6)); /*0x10316e1b4*/
      v14 = v11 + 1; /*0x10316e1b8*/
      if ( _bittest64(&v13, v11++) ) /*0x10316e1bc*/
      {
        v8 = v14 - 1; /*0x10316e1c8*/
        break; /*0x10316e1c8*/
      }
    }
  }
LABEL_22:
  for ( i = *((unsigned __int8 **)this + 1); i != *((unsigned __int8 **)this + 2); i += 136 ) /*0x10316e1d0*/
  {
    v17 = *i; /*0x10316e1dd*/
    if ( (v17 & 1) != 0 ) /*0x10316e1e3*/
      v18 = *((_QWORD *)i + 1); /*0x10316e1e9*/
    else
      v18 = v17 >> 1; /*0x10316e1e5*/
    v10 += v18; /*0x10316e1ed*/
  }
  v19 = (char *)this + 680; /*0x10316e1f8*/
  std::string::reserve(a1: (char *)this + 680, a2: v10); /*0x10316e202*/
  v20 = 100; /*0x10316e20e*/
  if ( (_BYTE)FFlag::DebugLuauUserDefinedClasses == 0 ) /*0x10316e213*/
  {
    v20 = 14; /*0x10316e215*/
    if ( (_BYTE)FFlag::LuauCompileFastpcall == 0 ) /*0x10316e21e*/
    {
      v20 = 13; /*0x10316e220*/
      if ( (_BYTE)FFlag::LuauCompileEmitVectorDouble == 0 ) /*0x10316e229*/
      {
        v20 = 12; /*0x10316e22b*/
        if ( (_BYTE)FFlag::LuauBytecodeCostModel == 0 ) /*0x10316e234*/
          v20 = (2 * FFlag::LuauEmitCallFeedback) | 9; /*0x10316e23e*/
      }
    }
  }
  std::string::operator=(a1: (char *)this + 680, a2: v20); /*0x10316e246*/
  v41 = 3; /*0x10316e24f*/
  std::string::append(a1: (char *)this + 680, a2: &v41, a3: 1); /*0x10316e25a*/
  Luau::BytecodeBuilder::writeStringTable(a1: this, a2: (char *)this + 680); /*0x10316e265*/
  v21 = *((_QWORD *)this + 65); /*0x10316e26a*/
  v22 = *((_QWORD *)this + 66); /*0x10316e271*/
  if ( (unsigned int)((unsigned __int64)(v22 - v21) >> 5) != 0 ) /*0x10316e284*/
  {
    v23 = 0; /*0x10316e28a*/
    do /*0x10316e31c*/
    {
      if ( *(_BYTE *)(v21 + 32 * v23 + 28) == 1 ) /*0x10316e29d*/
      {
        v45 = v23 + 1; /*0x10316e2a3*/
        std::string::append(a1: v19, a2: &v45, a3: 1); /*0x10316e2b2*/
        v24 = *(unsigned int *)(*((_QWORD *)v6 + 65) + 32 * v23 + 24); /*0x10316e2be*/
        v25 = v24; /*0x10316e2c2*/
        do /*0x10316e2f6*/
        {
          v44 = v24 & 0x7F | ((v24 > 0x7F) << 7); /*0x10316e2d8*/
          std::string::append(a1: v19, a2: &v44, a3: 1); /*0x10316e2e6*/
          v25 >>= 7; /*0x10316e2eb*/
          v12 = v24 <= 0x7F; /*0x10316e2ef*/
          v24 = v25; /*0x10316e2f3*/
        }
        while ( !v12 ); /*0x10316e2f6*/
        v6 = this; /*0x10316e2f8*/
        v21 = *((_QWORD *)this + 65); /*0x10316e2fc*/
        v22 = *((_QWORD *)this + 66); /*0x10316e303*/
      }
      ++v23; /*0x10316e30a*/
    }
    while ( v23 < (unsigned int)((unsigned __int64)(v22 - v21) >> 5) ); /*0x10316e31c*/
  }
  v40 = 0; /*0x10316e326*/
  std::string::append(a1: v19, a2: &v40, a3: 1); /*0x10316e331*/
  v26 = -252645135 * (unsigned int)((*((_QWORD *)v6 + 2) - *((_QWORD *)v6 + 1)) >> 3); /*0x10316e342*/
  v27 = v26; /*0x10316e34d*/
  do /*0x10316e382*/
  {
    v46[0] = v26 & 0x7F | ((v26 > 0x7F) << 7); /*0x10316e364*/
    std::string::append(a1: v19, a2: v46, a3: 1); /*0x10316e372*/
    v27 >>= 7; /*0x10316e377*/
    v12 = v26 <= 0x7F; /*0x10316e37b*/
    v26 = v27; /*0x10316e37f*/
  }
  while ( !v12 ); /*0x10316e382*/
  v28 = *((unsigned __int8 **)this + 1); /*0x10316e388*/
  for ( j = *((unsigned __int8 **)this + 2); v28 != j; v28 += 136 ) /*0x10316e397*/
  {
    if ( (_BYTE)FFlag::LuauBytecodeCostModel != 0 /*0x10316e3c6*/
      || (_BYTE)FFlag::LuauCompileEmitVectorDouble != 0
      || (_BYTE)FFlag::LuauCompileFastpcall != 0
      || (_BYTE)FFlag::DebugLuauUserDefinedClasses == 1 )
    {
      v29 = *v28; /*0x10316e3c8*/
      if ( (v29 & 1) != 0 ) /*0x10316e3d1*/
        v30 = *((_QWORD *)v28 + 1); /*0x10316e3d8*/
      else
        v30 = v29 >> 1; /*0x10316e3d3*/
      v31 = v30; /*0x10316e3dd*/
      do /*0x10316e412*/
      {
        v42 = v30 & 0x7F | ((v30 > 0x7F) << 7); /*0x10316e3f4*/
        std::string::append(a1: v19, a2: &v42, a3: 1); /*0x10316e402*/
        v31 >>= 7; /*0x10316e407*/
        v12 = v30 <= 0x7F; /*0x10316e40b*/
        v30 = v31; /*0x10316e40f*/
      }
      while ( !v12 ); /*0x10316e412*/
    }
    v32 = *v28; /*0x10316e414*/
    if ( (v32 & 1) != 0 ) /*0x10316e41c*/
    {
      v33 = *((unsigned __int8 **)v28 + 2); /*0x10316e41e*/
      v34 = *((_QWORD *)v28 + 1); /*0x10316e423*/
    }
    else
    {
      v33 = v28 + 1; /*0x10316e42a*/
      v34 = v32 >> 1; /*0x10316e42f*/
    }
    std::string::append(a1: v19, a2: v33, a3: v34); /*0x10316e434*/
  }
  v35 = *((unsigned int *)this + 9); /*0x10316e44e*/
  v36 = v35; /*0x10316e456*/
  do /*0x10316e48b*/
  {
    v43 = v35 & 0x7F | ((v35 > 0x7F) << 7); /*0x10316e46d*/
    result = std::string::append(a1: v19, a2: &v43, a3: 1); /*0x10316e47b*/
    v36 >>= 7; /*0x10316e480*/
    v12 = v35 <= 0x7F; /*0x10316e484*/
    v35 = v36; /*0x10316e488*/
  }
  while ( !v12 ); /*0x10316e48b*/
  return result; /*0x10316e48d*/
}

// Address: 0x10316e49c | Name: __ZN4Luau15BytecodeBuilder10getVersionEv
__int64 __fastcall Luau::BytecodeBuilder::getVersion(Luau::BytecodeBuilder *this)
{
  __int64 result; // rax

  LOBYTE(result) = 100; /*0x10316e4a7*/
  if ( (_BYTE)FFlag::DebugLuauUserDefinedClasses == 0 ) /*0x10316e4ac*/
  {
    LOBYTE(result) = 14; /*0x10316e4ae*/
    if ( (_BYTE)FFlag::LuauCompileFastpcall == 0 ) /*0x10316e4b7*/
    {
      LOBYTE(result) = 13; /*0x10316e4b9*/
      if ( (_BYTE)FFlag::LuauCompileEmitVectorDouble == 0 ) /*0x10316e4c2*/
      {
        LOBYTE(result) = 12; /*0x10316e4c4*/
        if ( (_BYTE)FFlag::LuauBytecodeCostModel == 0 ) /*0x10316e4cd*/
          LOBYTE(result) = (2 * FFlag::LuauEmitCallFeedback) | 9; /*0x10316e4d7*/
      }
    }
  }
  return (unsigned __int8)result; /*0x10316e4dc*/
}

// Address: 0x10316e4de | Name: __ZN4Luau15BytecodeBuilder22getTypeEncodingVersionEv
__int64 __fastcall Luau::BytecodeBuilder::getTypeEncodingVersion(Luau::BytecodeBuilder *this)
{
  return 3; /*0x10316e4e7*/
}

// Address: 0x10316e4ea | Name: __ZNK4Luau15BytecodeBuilder16writeStringTableERNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEE
void __fastcall Luau::BytecodeBuilder::writeStringTable(_QWORD *a1, __int64 a2)
{
  __int64 v4; // rax
  unsigned __int64 v5; // rcx
  __int64 v6; // rsi
  unsigned __int64 v7; // rdx
  unsigned __int64 v8; // rdx
  __int64 v9; // rdi
  unsigned __int64 v10; // rsi
  bool v11; // cf
  unsigned __int64 v12; // r15
  _QWORD *v13; // r14
  _BYTE *v14; // r12
  unsigned __int64 v15; // r13
  bool v16; // cc
  void *v17; // [rsp+8h] [rbp-48h] BYREF
  _BYTE *v18; // [rsp+10h] [rbp-40h]
  _BYTE v19[41]; // [rsp+27h] [rbp-29h] BYREF

  std::vector<Luau::BytecodeBuilder::StringRef>::vector[abi:ne200100](a1: &v17, a2: a1[73]); /*0x10316e50c*/
  v4 = a1[72]; /*0x10316e511*/
  if ( v4 != 0 ) /*0x10316e51b*/
  {
    v5 = 0; /*0x10316e524*/
    while ( 1 ) /*0x10316e52d*/
    {
      v6 = *(_QWORD *)(a1[71] + 8 * (v5 >> 6)); /*0x10316e52d*/
      if ( _bittest64(&v6, v5) ) /*0x10316e535*/
        break; /*0x10316e535*/
      if ( v4 == ++v5 ) /*0x10316e53d*/
        goto LABEL_13; /*0x10316e53d*/
    }
  }
  else
  {
    v5 = 0; /*0x10316e541*/
  }
LABEL_12:
  while ( v5 != v4 ) /*0x10316e5a8*/
  {
    *((_OWORD *)v17 + (unsigned int)(*(_DWORD *)(a1[68] + 24 * v5 + 16) - 1)) = *(_OWORD *)(a1[68] + 24 * v5); /*0x10316e562*/
    v7 = v5; /*0x10316e567*/
    v5 = a1[72]; /*0x10316e56a*/
    v8 = v7 + 1; /*0x10316e571*/
    if ( v5 <= v8 ) /*0x10316e577*/
      v5 = v8; /*0x10316e577*/
    while ( v5 != v8 ) /*0x10316e57e*/
    {
      v9 = *(_QWORD *)(a1[71] + 8 * (v8 >> 6)); /*0x10316e58e*/
      v10 = v8 + 1; /*0x10316e592*/
      v11 = _bittest64(&v9, v8++); /*0x10316e596*/
      if ( v11 ) /*0x10316e59d*/
      {
        v5 = v10 - 1; /*0x10316e5a2*/
        goto LABEL_12; /*0x10316e5a2*/
      }
    }
  }
LABEL_13:
  v12 = (unsigned int)((unsigned __int64)(v18 - (_BYTE *)v17) >> 4); /*0x10316e5aa*/
  do /*0x10316e5f5*/
  {
    v19[0] = v12 & 0x7F | ((v12 > 0x7F) << 7); /*0x10316e5d1*/
    std::string::append(a1: a2, a2: v19, a3: 1); /*0x10316e5df*/
    v11 = v12 < 0x80; /*0x10316e5eb*/
    v12 >>= 7; /*0x10316e5f2*/
  }
  while ( !v11 ); /*0x10316e5f5*/
  v13 = v17; /*0x10316e5f7*/
  v14 = v18; /*0x10316e5fb*/
  if ( v17 != v18 ) /*0x10316e602*/
  {
    do /*0x10316e659*/
    {
      v15 = *((unsigned int *)v13 + 2); /*0x10316e608*/
      do /*0x10316e641*/
      {
        v19[0] = v15 & 0x7F | ((v15 > 0x7F) << 7); /*0x10316e620*/
        std::string::append(a1: a2, a2: v19, a3: 1); /*0x10316e62e*/
        v16 = v15 <= 0x7F; /*0x10316e63a*/
        v15 >>= 7; /*0x10316e63e*/
      }
      while ( !v16 ); /*0x10316e641*/
      std::string::append(a1: a2, a2: *v13, a3: v13[1]); /*0x10316e64d*/
      v13 += 2; /*0x10316e652*/
    }
    while ( v13 != (_QWORD *)v14 ); /*0x10316e659*/
    v13 = v17; /*0x10316e65b*/
  }
  if ( v13 != nullptr ) /*0x10316e662*/
  {
    v18 = v13; /*0x10316e664*/
    operator delete(a1: v13); /*0x10316e66b*/
  }
}

// Address: 0x10316e6a0 | Name: __ZNK4Luau15BytecodeBuilder15writeClassShapeERNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEERKNS0_10ClassShapeE
__int64 __fastcall Luau::BytecodeBuilder::writeClassShape(__int64 a1, __int64 a2, _QWORD *a3)
{
  unsigned __int64 v5; // r12
  unsigned __int64 v6; // r14
  bool v7; // cc
  unsigned __int64 v8; // r12
  unsigned __int64 v9; // r14
  unsigned __int64 v10; // r12
  unsigned __int64 v11; // r14
  int *v12; // r12
  unsigned __int64 v13; // r13
  unsigned __int64 v14; // r14
  int *v15; // r15
  __int64 result; // rax
  unsigned __int64 v17; // r12
  unsigned __int64 v18; // r13
  _QWORD *v19; // [rsp+0h] [rbp-40h]
  int *i; // [rsp+8h] [rbp-38h]
  int *j; // [rsp+8h] [rbp-38h]
  char v22; // [rsp+13h] [rbp-2Dh] BYREF
  char v23; // [rsp+14h] [rbp-2Ch] BYREF
  char v24; // [rsp+15h] [rbp-2Bh] BYREF
  char v25; // [rsp+16h] [rbp-2Ah] BYREF
  _BYTE v26[41]; // [rsp+17h] [rbp-29h] BYREF

  v5 = *(int *)a3; /*0x10316e6b7*/
  v6 = v5; /*0x10316e6be*/
  do /*0x10316e6f3*/
  {
    v26[0] = v5 & 0x7F | ((v5 > 0x7F) << 7); /*0x10316e6d5*/
    std::string::append(a1: a2, a2: v26, a3: 1); /*0x10316e6e3*/
    v6 >>= 7; /*0x10316e6e8*/
    v7 = v5 <= 0x7F; /*0x10316e6ec*/
    v5 = v6; /*0x10316e6f0*/
  }
  while ( !v7 ); /*0x10316e6f3*/
  v8 = (__int64)(a3[2] - a3[1]) >> 2; /*0x10316e6fd*/
  v9 = v8; /*0x10316e705*/
  do /*0x10316e73a*/
  {
    v25 = v8 & 0x7F | ((v8 > 0x7F) << 7); /*0x10316e71c*/
    std::string::append(a1: a2, a2: &v25, a3: 1); /*0x10316e72a*/
    v9 >>= 7; /*0x10316e72f*/
    v7 = v8 <= 0x7F; /*0x10316e733*/
    v8 = v9; /*0x10316e737*/
  }
  while ( !v7 ); /*0x10316e73a*/
  v10 = (__int64)(a3[5] - a3[4]) >> 2; /*0x10316e744*/
  v11 = v10; /*0x10316e74c*/
  do /*0x10316e781*/
  {
    v24 = v10 & 0x7F | ((v10 > 0x7F) << 7); /*0x10316e763*/
    std::string::append(a1: a2, a2: &v24, a3: 1); /*0x10316e771*/
    v11 >>= 7; /*0x10316e776*/
    v7 = v10 <= 0x7F; /*0x10316e77a*/
    v10 = v11; /*0x10316e77e*/
  }
  while ( !v7 ); /*0x10316e781*/
  v12 = (int *)a3[1]; /*0x10316e783*/
  v19 = a3; /*0x10316e787*/
  for ( i = (int *)a3[2]; v12 != i; ++v12 ) /*0x10316e796*/
  {
    v13 = *v12; /*0x10316e79c*/
    v14 = v13; /*0x10316e7a0*/
    do /*0x10316e7d5*/
    {
      v23 = v13 & 0x7F | ((v13 > 0x7F) << 7); /*0x10316e7b7*/
      std::string::append(a1: a2, a2: &v23, a3: 1); /*0x10316e7c5*/
      v14 >>= 7; /*0x10316e7ca*/
      v7 = v13 <= 0x7F; /*0x10316e7ce*/
      v13 = v14; /*0x10316e7d2*/
    }
    while ( !v7 ); /*0x10316e7d5*/
  }
  v15 = (int *)v19[4]; /*0x10316e7e5*/
  result = v19[5]; /*0x10316e7e9*/
  for ( j = (int *)result; v15 != j; ++v15 ) /*0x10316e7f4*/
  {
    v17 = *v15; /*0x10316e7fa*/
    v18 = v17; /*0x10316e7fd*/
    do /*0x10316e832*/
    {
      v22 = v17 & 0x7F | ((v17 > 0x7F) << 7); /*0x10316e814*/
      result = std::string::append(a1: a2, a2: &v22, a3: 1); /*0x10316e822*/
      v18 >>= 7; /*0x10316e827*/
      v7 = v17 <= 0x7F; /*0x10316e82b*/
      v17 = v18; /*0x10316e82f*/
    }
    while ( !v7 ); /*0x10316e832*/
  }
  return result; /*0x10316e83e*/
}

// Address: 0x10316e84e | Name: __ZNK4Luau15BytecodeBuilder13writeLineInfoERNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEE
void __fastcall Luau::BytecodeBuilder::writeLineInfo(
        Luau::BytecodeBuilder *this,
        __int64 a2,
        __int64 a3,
        unsigned __int64 a4)
{
  Luau::BytecodeBuilder *v4; // r14
  __int64 v5; // r13
  int v6; // r15d
  __int64 v7; // rax
  unsigned __int64 v8; // rdx
  unsigned __int64 v9; // rsi
  unsigned __int64 v10; // rdi
  int v11; // r10d
  unsigned __int64 v12; // r9
  int v13; // r11d
  int v14; // ebx
  unsigned __int64 v15; // rdi
  char v16; // cl
  unsigned __int64 v17; // r13
  int *v18; // r13
  char v19; // al
  __int64 v20; // rcx
  __int64 v21; // rsi
  unsigned __int64 v22; // rsi
  unsigned __int64 v23; // rax
  int v24; // r8d
  unsigned __int64 v25; // rdi
  unsigned __int64 v26; // rdx
  unsigned __int64 v27; // r9
  char v28; // r12
  __int64 v29; // rax
  Luau::BytecodeBuilder *v30; // r15
  unsigned __int64 v31; // rbx
  char v32; // cl
  unsigned __int64 v33; // rbx
  int v34; // eax
  unsigned __int64 v35; // [rsp+8h] [rbp-68h]
  void *v36[2]; // [rsp+10h] [rbp-60h] BYREF
  __int64 v37; // [rsp+20h] [rbp-50h]
  int v38; // [rsp+34h] [rbp-3Ch] BYREF
  __int64 v39; // [rsp+38h] [rbp-38h]
  int v40; // [rsp+40h] [rbp-30h] BYREF
  char v41; // [rsp+47h] [rbp-29h]

  v39 = a2; /*0x10316e85f*/
  v4 = this; /*0x10316e863*/
  v5 = *((_QWORD *)this + 10); /*0x10316e86c*/
  v41 = FFlag::LuauVirtualBcBuilder; /*0x10316e872*/
  if ( (_BYTE)FFlag::LuauVirtualBcBuilder != 0 ) /*0x10316e875*/
  {
    v6 = Luau::BytecodeBuilder::calcLinesSpan(this); /*0x10316e87f*/
    v7 = *((_QWORD *)v4 + 9); /*0x10316e882*/
  }
  else
  {
    v7 = *((_QWORD *)this + 9); /*0x10316e88b*/
    if ( v5 == v7 ) /*0x10316e895*/
    {
      v6 = 0x1000000; /*0x10316e931*/
      v7 = v5; /*0x10316e937*/
    }
    else
    {
      v8 = (v5 - v7) >> 2; /*0x10316e89b*/
      v6 = 0x1000000; /*0x10316e89f*/
      v9 = 0; /*0x10316e8a5*/
      do /*0x10316e929*/
      {
        a4 = v9 + v6; /*0x10316e8aa*/
        v10 = a4; /*0x10316e8b1*/
        if ( v9 > a4 ) /*0x10316e8b4*/
          v10 = v9; /*0x10316e8b4*/
        v11 = *(_DWORD *)(v7 + 4 * v9); /*0x10316e8b8*/
        v12 = v9; /*0x10316e8bc*/
        v13 = v11; /*0x10316e8bf*/
        while ( v12 < a4 ) /*0x10316e8c5*/
        {
          v14 = *(_DWORD *)(v7 + 4 * v12); /*0x10316e8c7*/
          if ( v14 < v13 ) /*0x10316e8ce*/
            v13 = *(_DWORD *)(v7 + 4 * v12); /*0x10316e8ce*/
          if ( v11 <= v14 ) /*0x10316e8d5*/
            v11 = *(_DWORD *)(v7 + 4 * v12); /*0x10316e8d5*/
          if ( v11 - v13 > 255 ) /*0x10316e8e5*/
          {
            v10 = v12; /*0x10316e8f1*/
            break; /*0x10316e8f1*/
          }
          if ( ++v12 >= v8 ) /*0x10316e8ed*/
            goto LABEL_21; /*0x10316e8ed*/
        }
        v15 = v10 - v9; /*0x10316e8f4*/
        if ( v15 < v6 ) /*0x10316e8fa*/
        {
          v16 = -1; /*0x10316e8fc*/
          do /*0x10316e90f*/
            ++v16; /*0x10316e901*/
          while ( 2 << v16 <= (int)v15 ); /*0x10316e90f*/
          v6 = 1 << v16; /*0x10316e917*/
          a4 = (1 << v16) + v9; /*0x10316e920*/
        }
LABEL_21:
        v9 = a4; /*0x10316e923*/
      }
      while ( a4 < v8 ); /*0x10316e929*/
    }
  }
  v38 = 0; /*0x10316e93a*/
  *(_OWORD *)v36 = 0; /*0x10316e944*/
  v37 = 0; /*0x10316e948*/
  v17 = ((v5 - v7) >> 2) - 1; /*0x10316e957*/
  v35 = v17 / v6 + 1; /*0x10316e96c*/
  if ( v35 < 2 ) /*0x10316e970*/
  {
    v18 = &v38; /*0x10316e98a*/
    v19 = v41; /*0x10316e98e*/
  }
  else
  {
    std::vector<int>::resize(a1: v36, a2: v17 / v6 + 1); /*0x10316e979*/
    v18 = (int *)v36[0]; /*0x10316e97e*/
    v19 = FFlag::LuauVirtualBcBuilder; /*0x10316e982*/
  }
  if ( (v19 & 1) != 0 ) /*0x10316e993*/
  {
    Luau::BytecodeBuilder::fillBaselineInfo(this: v4, a2: v6, a3: v18, a4); /*0x10316e99e*/
  }
  else
  {
    v20 = *((_QWORD *)v4 + 9); /*0x10316e9a5*/
    v21 = *((_QWORD *)v4 + 10) - v20; /*0x10316e9ad*/
    if ( v21 != 0 ) /*0x10316e9b0*/
    {
      v22 = v21 >> 2; /*0x10316e9b2*/
      v23 = 0; /*0x10316e9b6*/
      do /*0x10316e9f5*/
      {
        v24 = *(_DWORD *)(v20 + 4 * v23); /*0x10316e9b8*/
        v25 = v23 + v6; /*0x10316e9bc*/
        v26 = v25; /*0x10316e9c3*/
        if ( v22 < v25 ) /*0x10316e9c6*/
          v26 = v22; /*0x10316e9c6*/
        if ( v23 < v26 ) /*0x10316e9cd*/
        {
          v27 = v23; /*0x10316e9cf*/
          do /*0x10316e9e3*/
          {
            if ( *(_DWORD *)(v20 + 4 * v27) < v24 ) /*0x10316e9d9*/
              v24 = *(_DWORD *)(v20 + 4 * v27); /*0x10316e9d9*/
            ++v27; /*0x10316e9dd*/
          }
          while ( v27 < v26 ); /*0x10316e9e3*/
        }
        v18[v23 / v6] = v24; /*0x10316e9ea*/
        v23 += v6; /*0x10316e9ef*/
      }
      while ( v25 < v22 ); /*0x10316e9f5*/
    }
  }
  v28 = -1; /*0x10316e9f7*/
  do /*0x10316ea0e*/
    ++v28; /*0x10316e9fe*/
  while ( 2 << v28 <= v6 ); /*0x10316ea0e*/
  LOBYTE(v40) = v28; /*0x10316ea14*/
  std::string::append(a1: v39, a2: &v40, a3: 1); /*0x10316ea20*/
  v29 = *((_QWORD *)v4 + 9); /*0x10316ea25*/
  if ( *((_QWORD *)v4 + 10) != v29 ) /*0x10316ea2d*/
  {
    v30 = v4; /*0x10316ea2f*/
    v31 = 0; /*0x10316ea32*/
    LOBYTE(v4) = 0; /*0x10316ea34*/
    do /*0x10316ea7b*/
    {
      v32 = (char)v4; /*0x10316ea40*/
      LODWORD(v4) = *(_DWORD *)(v29 + 4 * v31) - v18[v31 >> v28]; /*0x10316ea47*/
      LOBYTE(v40) = *(_BYTE *)(v29 + 4 * v31) - LOBYTE(v18[v31 >> v28]) - v32; /*0x10316ea51*/
      std::string::append(a1: v39, a2: &v40, a3: 1); /*0x10316ea61*/
      ++v31; /*0x10316ea66*/
      v29 = *((_QWORD *)v30 + 9); /*0x10316ea69*/
    }
    while ( v31 < (*((_QWORD *)v30 + 10) - v29) >> 2 ); /*0x10316ea7b*/
  }
  if ( v35 != 0 ) /*0x10316ea84*/
  {
    v33 = 0; /*0x10316ea86*/
    v34 = 0; /*0x10316ea8c*/
    do /*0x10316eab4*/
    {
      v40 = v18[v33] - v34; /*0x10316ea95*/
      std::string::append(a1: v39, a2: &v40, a3: 4); /*0x10316eaa4*/
      v34 = v18[v33++]; /*0x10316eaa9*/
    }
    while ( v33 < v35 ); /*0x10316eab4*/
  }
  if ( v36[0] != nullptr ) /*0x10316eabd*/
  {
    v36[1] = v36[0]; /*0x10316eabf*/
    operator delete(a1: v36[0]); /*0x10316eac3*/
  }
}

// Address: 0x10316eafa | Name: __ZNK4Luau15BytecodeBuilder13calcLinesSpanEv
__int64 __fastcall Luau::BytecodeBuilder::calcLinesSpan(Luau::BytecodeBuilder *this)
{
  __int64 v1; // rdx
  __int64 v2; // rsi
  unsigned __int64 v3; // rsi
  __int64 result; // rax
  unsigned __int64 v5; // rdi
  unsigned __int64 v6; // rcx
  unsigned __int64 v7; // r8
  int v8; // r11d
  unsigned __int64 v9; // r10
  int v10; // ebx
  int v11; // r14d
  unsigned __int64 v12; // r8
  char v13; // cl

  v1 = *((_QWORD *)this + 9); /*0x10316eafa*/
  v2 = *((_QWORD *)this + 10) - v1; /*0x10316eb02*/
  if ( v2 == 0 ) /*0x10316eb05*/
    return 0x1000000; /*0x10316eba7*/
  v3 = v2 >> 2; /*0x10316eb12*/
  result = 0x1000000; /*0x10316eb16*/
  v5 = 0; /*0x10316eb1b*/
  do /*0x10316eb9c*/
  {
    v6 = v5 + (int)result; /*0x10316eb20*/
    v7 = v6; /*0x10316eb27*/
    if ( v5 > v6 ) /*0x10316eb2a*/
      v7 = v5; /*0x10316eb2a*/
    v8 = *(_DWORD *)(v1 + 4 * v5); /*0x10316eb2e*/
    v9 = v5; /*0x10316eb32*/
    v10 = v8; /*0x10316eb35*/
    while ( v9 < v6 ) /*0x10316eb3b*/
    {
      v11 = *(_DWORD *)(v1 + 4 * v9); /*0x10316eb3d*/
      if ( v11 < v10 ) /*0x10316eb44*/
        v10 = *(_DWORD *)(v1 + 4 * v9); /*0x10316eb44*/
      if ( v8 <= v11 ) /*0x10316eb4b*/
        v8 = *(_DWORD *)(v1 + 4 * v9); /*0x10316eb4b*/
      if ( v8 - v10 > 255 ) /*0x10316eb5c*/
      {
        v7 = v9; /*0x10316eb68*/
        break; /*0x10316eb68*/
      }
      if ( ++v9 >= v3 ) /*0x10316eb64*/
        goto LABEL_19; /*0x10316eb64*/
    }
    v12 = v7 - v5; /*0x10316eb6b*/
    if ( v12 < (int)result ) /*0x10316eb71*/
    {
      v13 = -1; /*0x10316eb73*/
      do /*0x10316eb84*/
        ++v13; /*0x10316eb78*/
      while ( 2 << v13 <= (int)v12 ); /*0x10316eb84*/
      result = (unsigned int)(1 << v13); /*0x10316eb8b*/
      v6 = (int)result + v5; /*0x10316eb93*/
    }
LABEL_19:
    v5 = v6; /*0x10316eb96*/
  }
  while ( v6 < v3 ); /*0x10316eb9c*/
  return result; /*0x10316eba6*/
}

// Address: 0x10316ebae | Name: __ZNK4Luau15BytecodeBuilder16fillBaselineInfoEiPim
void __fastcall Luau::BytecodeBuilder::fillBaselineInfo(Luau::BytecodeBuilder *this, int a2, int *a3)
{
  __int64 v3; // r8
  __int64 v4; // rdi
  unsigned __int64 v6; // rdi
  unsigned __int64 v7; // rax
  int v8; // r10d
  unsigned __int64 v9; // r9
  unsigned __int64 v10; // rdx
  unsigned __int64 v11; // r11

  v3 = *((_QWORD *)this + 9); /*0x10316ebae*/
  v4 = *((_QWORD *)this + 10) - v3; /*0x10316ebb6*/
  if ( v4 != 0 ) /*0x10316ebb9*/
  {
    v6 = v4 >> 2; /*0x10316ebc3*/
    v7 = 0; /*0x10316ebca*/
    do /*0x10316ec08*/
    {
      v8 = *(_DWORD *)(v3 + 4 * v7); /*0x10316ebcc*/
      v9 = v7 + a2; /*0x10316ebd0*/
      v10 = v9; /*0x10316ebd7*/
      if ( v6 < v9 ) /*0x10316ebda*/
        v10 = v6; /*0x10316ebda*/
      if ( v7 < v10 ) /*0x10316ebe1*/
      {
        v11 = v7; /*0x10316ebe3*/
        do /*0x10316ebf7*/
        {
          if ( *(_DWORD *)(v3 + 4 * v11) < v8 ) /*0x10316ebed*/
            v8 = *(_DWORD *)(v3 + 4 * v11); /*0x10316ebed*/
          ++v11; /*0x10316ebf1*/
        }
        while ( v11 < v10 ); /*0x10316ebf7*/
      }
      a3[v7 / a2] = v8; /*0x10316ebfe*/
      v7 += a2; /*0x10316ec02*/
    }
    while ( v9 < v6 ); /*0x10316ec08*/
  }
}

// Address: 0x10316ec0e | Name: __ZN4Luau15BytecodeBuilder11getImportIdEi
__int64 __fastcall Luau::BytecodeBuilder::getImportId(Luau::BytecodeBuilder *this)
{
  return ((_DWORD)this << 20) | 0x40000000u; /*0x10316ec1c*/
}

// Address: 0x10316ec1e | Name: __ZN4Luau15BytecodeBuilder11getImportIdEii
__int64 __fastcall Luau::BytecodeBuilder::getImportId(Luau::BytecodeBuilder *this, int a2)
{
  return ((_DWORD)this << 20) | (a2 << 10) | 0x80000000; /*0x10316ec31*/
}

// Address: 0x10316ec34 | Name: __ZN4Luau15BytecodeBuilder11getImportIdEiii
__int64 __fastcall Luau::BytecodeBuilder::getImportId(Luau::BytecodeBuilder *this, int a2, int a3)
{
  return a3 | ((_DWORD)this << 20) | (a2 << 10) | 0xC0000000; /*0x10316ec49*/
}

// Address: 0x10316ec4c | Name: __ZN4Luau15BytecodeBuilder17decomposeImportIdEjRiS1_S1_
__int64 __fastcall Luau::BytecodeBuilder::decomposeImportId(
        Luau::BytecodeBuilder *this,
        unsigned int *a2,
        unsigned int *a3,
        int *a4,
        int *a5)
{
  __int64 result; // rax
  unsigned int v6; // r8d
  unsigned int v7; // r11d
  int v8; // edi

  result = (unsigned int)this >> 30; /*0x10316ec52*/
  v6 = ((unsigned int)this >> 20) & 0x3FF; /*0x10316ec62*/
  if ( (unsigned int)this < 0x40000000 ) /*0x10316ec71*/
    v6 = -1; /*0x10316ec71*/
  v7 = ((unsigned int)this >> 10) & 0x3FF; /*0x10316ec7c*/
  if ( (int)this >= 0 ) /*0x10316ec81*/
    v7 = -1; /*0x10316ec81*/
  *a2 = v6; /*0x10316ec85*/
  *a3 = v7; /*0x10316ec88*/
  v8 = (unsigned __int16)this & 0x3FF; /*0x10316ec8b*/
  if ( (_DWORD)result != 3 ) /*0x10316ec91*/
    v8 = -1; /*0x10316ec91*/
  *a4 = v8; /*0x10316ec95*/
  return result; /*0x10316ec97*/
}

// Address: 0x10316ec9a | Name: __ZN4Luau15BytecodeBuilder13getStringHashENS0_9StringRefE
__int64 __fastcall Luau::BytecodeBuilder::getStringHash(__int64 a1, __int64 a2)
{
  unsigned int v2; // ecx
  __int64 result; // rax

  if ( a2 == 0 ) /*0x10316eca1*/
    return 0; /*0x10316ecc3*/
  v2 = a2; /*0x10316eca3*/
  do /*0x10316ecbf*/
  {
    result = v2 ^ (32 * v2 + (v2 >> 2) + *(unsigned __int8 *)(a1 + a2 - 1)); /*0x10316ecb8*/
    v2 ^= 32 * v2 + (v2 >> 2) + *(unsigned __int8 *)(a1 + a2-- - 1); /*0x10316ecba*/
  }
  while ( a2 != 0 ); /*0x10316ecbf*/
  return result; /*0x10316ecc5*/
}

// Address: 0x10316ecc8 | Name: __ZN4Luau15BytecodeBuilder9foldJumpsEv
void __fastcall Luau::BytecodeBuilder::foldJumps(Luau::BytecodeBuilder *this)
{
  unsigned int *v1; // rax
  unsigned int *v2; // rcx
  __int64 v3; // rdx
  __int64 v4; // rsi
  int v5; // r8d
  __int64 v6; // rdi
  int v7; // r9d
  int v8; // r10d
  int v9; // r11d
  int v10; // r9d

  if ( *((_BYTE *)this + 240) == 0 ) /*0x10316eccf*/
  {
    v1 = *((unsigned int **)this + 18); /*0x10316ecd5*/
    v2 = *((unsigned int **)this + 19); /*0x10316ecdc*/
    if ( v1 != v2 ) /*0x10316ece6*/
    {
      v3 = *((_QWORD *)this + 6); /*0x10316ecf3*/
      while ( 1 ) /*0x10316ecf7*/
      {
        v4 = *v1; /*0x10316ecf7*/
        v5 = *(_DWORD *)(v3 + 4 * v4); /*0x10316ecf9*/
        v6 = (unsigned int)(v4 + (v5 >> 16) + 1); /*0x10316ed05*/
        v7 = *(_DWORD *)(v3 + 4 * v6); /*0x10316ed07*/
        v8 = (unsigned __int8)v7; /*0x10316ed0b*/
        v9 = v7 >> 16; /*0x10316ed19*/
        if ( (unsigned __int8)v7 == 23 && v7 >> 16 >= 0 ) /*0x10316ed1d*/
        {
          do /*0x10316ed40*/
          {
            v6 = (unsigned int)(v9 + v6 + 1); /*0x10316ed29*/
            v7 = *(_DWORD *)(v3 + 4 * v6); /*0x10316ed2b*/
            v8 = (unsigned __int8)v7; /*0x10316ed2f*/
            if ( (unsigned __int8)v7 != 23 ) /*0x10316ed37*/
              break; /*0x10316ed37*/
            v9 = v7 >> 16; /*0x10316ed3c*/
          }
          while ( v7 >> 16 >= 0 ); /*0x10316ed40*/
        }
        if ( (_BYTE)v5 == 23 && v8 == 22 ) /*0x10316ed4c*/
          goto LABEL_11; /*0x10316ed4c*/
        v10 = v6 + ~(_DWORD)v4; /*0x10316ed54*/
        if ( (__int16)(v6 + ~(_WORD)v4) == v10 ) /*0x10316ed5e*/
          break; /*0x10316ed5e*/
LABEL_12:
        v1[1] = v6; /*0x10316ed6f*/
        v1 += 2; /*0x10316ed72*/
        if ( v1 == v2 ) /*0x10316ed79*/
          return; /*0x10316ed79*/
      }
      v7 = (unsigned __int16)v5 | (v10 << 16); /*0x10316ed68*/
LABEL_11:
      *(_DWORD *)(v3 + 4 * v4) = v7; /*0x10316ed6b*/
      goto LABEL_12; /*0x10316ed6b*/
    }
  }
}

// Address: 0x10316ed84 | Name: __ZN4Luau15BytecodeBuilder11expandJumpsEv
Luau::BytecodeBuilder *__fastcall Luau::BytecodeBuilder::expandJumps(Luau::BytecodeBuilder *this, __int64 a2)
{
  __int64 v4; // rdi
  __int64 v5; // rsi
  unsigned __int64 v6; // rdx
  __int64 v7; // rdx
  _BYTE *v8; // rdi
  _BYTE *v9; // rax
  unsigned __int64 v10; // r15
  unsigned __int64 v11; // rbx
  __int64 v12; // rdi
  __int64 v13; // rax
  __int64 v14; // rcx
  int v15; // ecx
  unsigned int v16; // eax
  int OpLength; // eax
  __int64 v18; // rdx
  int v19; // r12d
  __int64 v20; // r15
  unsigned int *v21; // rcx
  unsigned int *v22; // rdx
  __int64 v23; // rsi
  _WORD *v24; // r8
  __int64 v25; // r11
  __int64 v26; // r9
  int v27; // ebx
  int v28; // r10d
  unsigned int v29; // r11d
  __int128 v30; // xmm0
  __int64 v31; // rax
  void *v32; // rax
  __int128 v33; // xmm0
  __int64 v34; // rcx
  __int64 v35; // rcx
  __int64 v36; // rdx
  __int64 v37; // rsi
  __int64 v38; // r8
  int v39; // r9d
  int v40; // r9d
  __int64 v41; // rcx
  __int64 v42; // rdx
  __int64 v43; // rsi
  __int64 v44; // r8
  int v45; // r9d
  int v46; // r9d
  unsigned int v48; // [rsp+8h] [rbp-78h]
  unsigned __int64 v49; // [rsp+8h] [rbp-78h]
  __int128 v50; // [rsp+10h] [rbp-70h] BYREF
  __int64 v51; // [rsp+20h] [rbp-60h]
  void *v52[2]; // [rsp+30h] [rbp-50h] BYREF
  __int64 v53; // [rsp+40h] [rbp-40h]
  _DWORD v54[11]; // [rsp+54h] [rbp-2Ch] BYREF

  if ( *(_BYTE *)(a2 + 240) != 0 ) /*0x10316ed9f*/
  {
    v4 = *(_QWORD *)(a2 + 144); /*0x10316eda8*/
    v5 = *(_QWORD *)(a2 + 152); /*0x10316edaf*/
    _BitScanReverse64(&v6, (v5 - v4) >> 3); /*0x10316edc0*/
    v7 = (2 * ((unsigned int)v6 ^ 0x3F)) ^ 0x7ELL; /*0x10316edc9*/
    if ( v5 == v4 ) /*0x10316edd2*/
      v7 = 0; /*0x10316edd2*/
    std::__introsort<std::_ClassicAlgPolicy,Luau::BytecodeBuilder::expandJumps(void)::$_0 &,Luau::BytecodeBuilder::Jump *,false>( /*0x10316eddb*/
      a1: v4,
      a2: v5,
      a3: v7,
      a4: 1);
    std::vector<unsigned int>::vector[abi:ne200100]( /*0x10316edef*/
      a1: this,
      a2: (__int64)(*(_QWORD *)(a2 + 56) - *(_QWORD *)(a2 + 48)) >> 2);
    v53 = 0; /*0x10316edf8*/
    *(_OWORD *)v52 = 0; /*0x10316edff*/
    v51 = 0; /*0x10316ee02*/
    v50 = 0; /*0x10316ee06*/
    std::vector<unsigned int>::reserve(a1: v52, a2: (__int64)(*(_QWORD *)(a2 + 56) - *(_QWORD *)(a2 + 48)) >> 2); /*0x10316ee16*/
    std::vector<int>::reserve(a1: &v50, a2: (__int64)(*(_QWORD *)(a2 + 56) - *(_QWORD *)(a2 + 48)) >> 2); /*0x10316ee2b*/
    v8 = *(_BYTE **)(a2 + 48); /*0x10316ee30*/
    v9 = *(_BYTE **)(a2 + 56); /*0x10316ee34*/
    if ( v9 != v8 ) /*0x10316ee3b*/
    {
      v10 = 0; /*0x10316ee41*/
      v11 = 0; /*0x10316ee44*/
      do /*0x10316ef4d*/
      {
        v12 = (unsigned __int8)v8[4 * v11]; /*0x10316ee46*/
        v13 = *(_QWORD *)(a2 + 144); /*0x10316ee4a*/
        if ( v10 < (*(_QWORD *)(a2 + 152) - v13) >> 3 ) /*0x10316ee62*/
        {
          v14 = *(unsigned int *)(v13 + 8 * v10); /*0x10316ee64*/
          if ( v11 == v14 ) /*0x10316ee6b*/
          {
            v48 = v12; /*0x10316ee6d*/
            v15 = *(_DWORD *)(v13 + 8 * v10 + 4) + ~(_DWORD)v14; /*0x10316ee72*/
            v16 = -v15; /*0x10316ee79*/
            if ( v15 > 0 ) /*0x10316ee7b*/
              v16 = v15; /*0x10316ee7b*/
            if ( v16 >= 0x2AAB ) /*0x10316ee83*/
            {
              v54[0] = 65559; /*0x10316ee85*/
              std::vector<unsigned int>::push_back[abi:ne200100](a1: v52, a2: v54); /*0x10316ee97*/
              v54[0] = 67; /*0x10316ee9c*/
              std::vector<unsigned int>::push_back[abi:ne200100](a1: v52, a2: v54); /*0x10316eeaa*/
              std::vector<int>::push_back[abi:ne200100](a1: &v50, a2: 4 * v11 + *(_QWORD *)(a2 + 72)); /*0x10316eec2*/
              std::vector<int>::push_back[abi:ne200100](a1: &v50, a2: *(_QWORD *)(a2 + 72) + 4 * v11); /*0x10316eed2*/
            }
            ++v10; /*0x10316eed7*/
            v12 = v48; /*0x10316eeda*/
          }
        }
        v49 = v10; /*0x10316eedd*/
        OpLength = Luau::getOpLength(a1: v12); /*0x10316eee1*/
        if ( OpLength > 0 ) /*0x10316eee8*/
        {
          v19 = OpLength; /*0x10316eeea*/
          v20 = 4 * v11; /*0x10316eeed*/
          do /*0x10316ef32*/
          {
            *(_DWORD *)(*(_QWORD *)this + 4 * v11) = (unsigned __int64)((char *)v52[1] - (char *)v52[0]) >> 2; /*0x10316ef05*/
            std::vector<unsigned int>::push_back[abi:ne200100](a1: v52, a2: v20 + *(_QWORD *)(a2 + 48), a3: v18); /*0x10316ef13*/
            std::vector<int>::push_back[abi:ne200100](a1: &v50, a2: v20 + *(_QWORD *)(a2 + 72)); /*0x10316ef23*/
            ++v11; /*0x10316ef28*/
            v20 += 4; /*0x10316ef2b*/
            --v19; /*0x10316ef2f*/
          }
          while ( v19 != 0 ); /*0x10316ef32*/
        }
        v8 = *(_BYTE **)(a2 + 48); /*0x10316ef34*/
        v9 = *(_BYTE **)(a2 + 56); /*0x10316ef38*/
        v10 = v49; /*0x10316ef49*/
      }
      while ( v11 < (v9 - v8) >> 2 ); /*0x10316ef4d*/
    }
    v21 = *(unsigned int **)(a2 + 144); /*0x10316ef53*/
    v22 = *(unsigned int **)(a2 + 152); /*0x10316ef5a*/
    if ( v21 != v22 ) /*0x10316ef64*/
    {
      v23 = *(_QWORD *)this; /*0x10316ef66*/
      v24 = v52[0]; /*0x10316ef6a*/
      do /*0x10316efd5*/
      {
        v25 = v21[1]; /*0x10316ef71*/
        v26 = *(unsigned int *)(v23 + 4LL * *v21); /*0x10316ef75*/
        v27 = v25 + ~*v21; /*0x10316ef7e*/
        v28 = *(_DWORD *)(v23 + 4 * v25) - v26; /*0x10316ef85*/
        v29 = -v27; /*0x10316ef8b*/
        if ( v27 > 0 ) /*0x10316ef8e*/
          v29 = v27; /*0x10316ef8e*/
        if ( v29 < 0x2AAB ) /*0x10316ef99*/
        {
          v24[2 * v26 + 1] = (unsigned int)((v28 << 16) - 0x10000) >> 16; /*0x10316efc8*/
        }
        else
        {
          *(_DWORD *)&v24[2 * (unsigned int)(v26 - 1)] = LOBYTE(v24[2 * (unsigned int)(v26 - 1)]) | (v28 << 8); /*0x10316efab*/
          v24[2 * v26 + 1] = -2; /*0x10316efaf*/
        }
        v21 += 2; /*0x10316efce*/
      }
      while ( v21 != v22 ); /*0x10316efd5*/
    }
    v30 = *(_OWORD *)v52; /*0x10316efd7*/
    v52[0] = v8; /*0x10316efdb*/
    *(_OWORD *)(a2 + 48) = v30; /*0x10316efdf*/
    v52[1] = v9; /*0x10316efe4*/
    v31 = *(_QWORD *)(a2 + 64); /*0x10316efe8*/
    *(_QWORD *)(a2 + 64) = v53; /*0x10316eff0*/
    v53 = v31; /*0x10316eff4*/
    v32 = *(void **)(a2 + 72); /*0x10316eff8*/
    v33 = v50; /*0x10316effc*/
    *(_QWORD *)&v50 = v32; /*0x10316f000*/
    *(_OWORD *)(a2 + 72) = v33; /*0x10316f004*/
    v34 = *(_QWORD *)(a2 + 88); /*0x10316f009*/
    *(_QWORD *)(a2 + 88) = v51; /*0x10316f011*/
    v51 = v34; /*0x10316f015*/
    v35 = *(_QWORD *)(a2 + 424); /*0x10316f019*/
    v36 = *(_QWORD *)(a2 + 432); /*0x10316f020*/
    if ( v35 != v36 ) /*0x10316f02a*/
    {
      v37 = *(_QWORD *)this; /*0x10316f02c*/
      do /*0x10316f060*/
      {
        v38 = *(unsigned int *)(v35 + 8); /*0x10316f030*/
        v39 = *(_DWORD *)(v35 + 12); /*0x10316f034*/
        if ( (_DWORD)v38 == v39 ) /*0x10316f03b*/
          v40 = *(_DWORD *)(v37 + 4 * v38); /*0x10316f03d*/
        else
          v40 = *(_DWORD *)(v37 + 4LL * (unsigned int)(v39 - 1)) + 1; /*0x10316f04a*/
        *(_DWORD *)(v35 + 12) = v40; /*0x10316f04d*/
        *(_DWORD *)(v35 + 8) = *(_DWORD *)(v37 + 4 * v38); /*0x10316f055*/
        v35 += 16; /*0x10316f059*/
      }
      while ( v35 != v36 ); /*0x10316f060*/
    }
    v41 = *(_QWORD *)(a2 + 472); /*0x10316f062*/
    v42 = *(_QWORD *)(a2 + 480); /*0x10316f069*/
    if ( v41 != v42 ) /*0x10316f073*/
    {
      v43 = *(_QWORD *)this; /*0x10316f075*/
      do /*0x10316f0a9*/
      {
        v44 = *(unsigned int *)(v41 + 8); /*0x10316f079*/
        v45 = *(_DWORD *)(v41 + 12); /*0x10316f07d*/
        if ( (_DWORD)v44 == v45 ) /*0x10316f084*/
          v46 = *(_DWORD *)(v43 + 4 * v44); /*0x10316f086*/
        else
          v46 = *(_DWORD *)(v43 + 4LL * (unsigned int)(v45 - 1)) + 1; /*0x10316f093*/
        *(_DWORD *)(v41 + 12) = v46; /*0x10316f096*/
        *(_DWORD *)(v41 + 8) = *(_DWORD *)(v43 + 4 * v44); /*0x10316f09e*/
        v41 += 16; /*0x10316f0a2*/
      }
      while ( v41 != v42 ); /*0x10316f0a9*/
    }
    if ( v32 != nullptr ) /*0x10316f0ae*/
    {
      *((_QWORD *)&v50 + 1) = v32; /*0x10316f0b0*/
      operator delete(a1: v32); /*0x10316f0b7*/
      v8 = v52[0]; /*0x10316f0bc*/
    }
    if ( v8 != nullptr ) /*0x10316f0c3*/
    {
      v52[1] = v8; /*0x10316f0c5*/
      operator delete(a1: v8); /*0x10316f0c9*/
    }
  }
  else
  {
    *(_OWORD *)this = 0; /*0x10316f0d3*/
    *((_QWORD *)this + 2) = 0; /*0x10316f0d8*/
  }
  return this; /*0x10316f0e3*/
}

// Address: 0x10316f13c | Name: __ZN4Luau15BytecodeBuilder8getErrorERKNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEE
__int64 __fastcall Luau::BytecodeBuilder::getError(__int64 a1, unsigned __int8 *a2)
{
  unsigned int v4; // edx
  __int64 v5; // rdx
  unsigned __int8 *v6; // r14

  *(_OWORD *)a1 = 0; /*0x10316f14c*/
  *(_QWORD *)(a1 + 16) = 0; /*0x10316f14f*/
  std::string::push_back(a1, a2: 0); /*0x10316f159*/
  v4 = *a2; /*0x10316f15e*/
  if ( (v4 & 1) != 0 ) /*0x10316f165*/
  {
    v5 = *((_QWORD *)a2 + 1); /*0x10316f167*/
    v6 = *((unsigned __int8 **)a2 + 2); /*0x10316f16b*/
  }
  else
  {
    v6 = a2 + 1; /*0x10316f171*/
    v5 = v4 >> 1; /*0x10316f174*/
  }
  std::string::append(a1, a2: v6, a3: v5); /*0x10316f17c*/
  return a1; /*0x10316f184*/
}

// Address: 0x10316f1a2 | Name: __ZNK4Luau15BytecodeBuilder13validateConstEi
void __fastcall Luau::BytecodeBuilder::validateConst(Luau::BytecodeBuilder *this)
{
  ; /*0x10316f1a2*/
}

// Address: 0x10316f1a8 | Name: __ZNK4Luau15BytecodeBuilder13validateConstEiNS0_8Constant4TypeE
void Luau::BytecodeBuilder::validateConst()
{
  ; /*0x10316f1a8*/
}

// Address: 0x10316f1ae | Name: __ZNK4Luau15BytecodeBuilder13validateProtoEi
__int64 __fastcall Luau::BytecodeBuilder::validateProto(Luau::BytecodeBuilder *this, int a2)
{
  return *(unsigned __int8 *)(*((_QWORD *)this + 1) + 136LL * *(unsigned int *)(*((_QWORD *)this + 15) + 4LL * a2) + 26); /*0x10316f1d0*/
}

// Address: 0x10316f1d2 | Name: __ZNK4Luau15BytecodeBuilder15validateClosureEi
__int64 __fastcall Luau::BytecodeBuilder::validateClosure(Luau::BytecodeBuilder *this, int a2)
{
  return *(unsigned __int8 *)(*((_QWORD *)this + 1) /*0x10316f1f9*/
                            + 136LL * *(unsigned int *)(*((_QWORD *)this + 12) + 40LL * a2 + 8)
                            + 26);
}

// Address: 0x10316f1fc | Name: __ZNK4Luau15BytecodeBuilder12dumpConstantERNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEEib
void __fastcall Luau::BytecodeBuilder::dumpConstant(_QWORD *a1, __int64 a2, int a3, char a4, int a5, int a6)
{
  __int64 v7; // rax
  __int64 v8; // rcx
  const char *v9; // rdx
  __int64 v12; // rcx
  const char *v13; // rsi
  __int64 v14; // rax
  char v15; // al
  unsigned int v16; // r15d
  int v17; // r13d
  __int64 v18; // rax
  int v19; // r8d
  int v20; // r9d
  __int64 v21; // rax
  __int64 v22; // rsi
  __int64 v23; // rax
  unsigned __int64 v24; // rdx
  _BYTE *v25; // rcx
  const char *v26; // rsi
  int v27; // edi
  double v28; // xmm3_8
  __int64 v29; // r12
  __int64 v30; // rax
  __int64 v31; // rdi
  unsigned __int64 v32; // rsi
  _QWORD *v33; // r12
  unsigned __int64 v34; // r13
  unsigned int v35; // edx
  int v36; // edi
  const char *v37; // rsi
  unsigned __int64 v38; // rax
  __int64 v39; // r12
  __int64 v40; // r13
  __int64 v41; // rax
  int v42; // edx
  unsigned __int64 v43; // rcx
  int v44; // r8d
  int v45; // r9d
  _QWORD *v46; // r13
  __int64 v47; // rax
  unsigned __int64 v48; // r15
  unsigned __int64 v49; // rcx
  int v50; // r8d
  int v51; // r9d
  __int64 v52; // rax
  unsigned __int64 v53; // r15
  __int64 v54; // r12
  int v55; // r13d
  char v56; // cl
  int v57; // eax
  int v58; // r15d
  __int64 v59; // rsi
  _DWORD *v60; // rdx
  __int64 v61; // rcx
  unsigned __int64 v62; // r8
  __int64 v63; // r9
  __int64 v64; // rax
  _DWORD *v65; // rsi
  unsigned __int64 v66; // rdi
  unsigned int v67; // r8d
  __int64 v68; // r8
  int v69; // edx
  int v70; // ecx
  int v71; // r8d
  int v72; // r9d
  unsigned __int64 v73; // r13
  int v74; // edx
  int v75; // ecx
  int v76; // r8d
  int v77; // r9d
  int v78; // edx
  int v79; // ecx
  int v80; // r8d
  int v81; // r9d
  int v82; // [rsp+Ch] [rbp-74h] BYREF
  void *v83[4]; // [rsp+10h] [rbp-70h] BYREF
  void *v84[4]; // [rsp+30h] [rbp-50h] BYREF
  int v85; // [rsp+54h] [rbp-2Ch]

  v7 = a1[12]; /*0x10316f213*/
  v8 = 5LL * a3; /*0x10316f217*/
  v9 = (const char *)*(unsigned int *)(v7 + 40LL * a3); /*0x10316f21b*/
  v12 = v7 + 8 * v8; /*0x10316f22e*/
  switch ( (unsigned __int64)v9 )
  {
    case 0uLL:
      v13 = "nil"; /*0x10316f242*/
      goto LABEL_97; /*0x10316f249*/
    case 1uLL:
      v9 = "false"; /*0x10316f3e2*/
      if ( *(_BYTE *)(v12 + 8) != 0 ) /*0x10316f3e9*/
        v9 = "true"; /*0x10316f3e9*/
      v13 = "%s"; /*0x10316f3ed*/
      goto LABEL_97; /*0x10316f3f4*/
    case 2uLL:
      Luau::formatAppend(a1: a2, a2: (unsigned int)"%.17g", a3: (_DWORD)v9, a4: v12, a5, a6); /*0x10316f2d7*/
      return; /*0x10316f2d7*/
    case 3uLL:
      v9 = *(const char **)(v12 + 8); /*0x10316f2dc*/
      v13 = "%lld"; /*0x10316f2e0*/
      goto LABEL_97; /*0x10316f2e7*/
    case 4uLL:
      if ( *(float *)(v12 + 20) == 0.0 ) /*0x10316f26b*/
        goto LABEL_82; /*0x10316f26b*/
      goto LABEL_96; /*0x10316f26b*/
    case 5uLL:
      v28 = *(double *)(v12 + 32); /*0x10316f3fe*/
      if ( (_BYTE)FFlag::LuauCompileEmitVectorDouble == 1 ) /*0x10316f40a*/
      {
        if ( v28 == 0.0 ) /*0x10316f422*/
          v13 = "%.17g, %.17g, %.17g"; /*0x10316f42e*/
        else
          v13 = "%.17g, %.17g, %.17g, %.17g"; /*0x10316f9d7*/
      }
      else if ( v28 == 0.0 ) /*0x10316f92e*/
      {
LABEL_82:
        v13 = "%.9g, %.9g, %.9g"; /*0x10316f93a*/
      }
      else
      {
LABEL_96:
        v13 = "%.9g, %.9g, %.9g, %.9g"; /*0x10316f9e4*/
      }
      goto LABEL_97; /*0x10316f435*/
    case 6uLL:
      v29 = a1[75]; /*0x10316f43f*/
      v30 = 16LL * (unsigned int)(*(_DWORD *)(v12 + 8) - 1); /*0x10316f446*/
      v25 = *(_BYTE **)(v29 + v30); /*0x10316f44a*/
      v24 = *(_QWORD *)(v29 + v30 + 8); /*0x10316f44e*/
      if ( v24 == 0 ) /*0x10316f456*/
        goto LABEL_99; /*0x10316f456*/
      if ( *v25 >= 0x20u ) /*0x10316f45f*/
      {
        v31 = 1; /*0x10316f461*/
        do /*0x10316f476*/
        {
          v32 = v31; /*0x10316f466*/
          if ( v24 == v31 ) /*0x10316f46c*/
            break; /*0x10316f46c*/
          ++v31; /*0x10316f46e*/
        }
        while ( v25[v32] > 0x1Fu ); /*0x10316f476*/
        if ( v32 >= v24 ) /*0x10316f47b*/
        {
          if ( v24 > 0x1F ) /*0x10316fa0b*/
          {
            v26 = "'%.*s'..."; /*0x10316fa2c*/
            v27 = a2; /*0x10316fa33*/
            LODWORD(v24) = 32; /*0x10316fa36*/
          }
          else
          {
LABEL_99:
            v26 = "'%.*s'"; /*0x10316fa0d*/
            v27 = a2; /*0x10316fa14*/
          }
LABEL_100:
          Luau::formatAppend(a1: v27, a2: (_DWORD)v26, a3: v24, a4: (_DWORD)v25, a5, a6); /*0x10316fa17*/
          return; /*0x10316fa27*/
        }
      }
      v33 = (_QWORD *)(v30 + v29); /*0x10316f481*/
      Luau::formatAppend(a1: a2, a2: (unsigned int)"'", a3: v24, a4: (_DWORD)v25, a5, a6); /*0x10316f490*/
      if ( v33[1] == 0 ) /*0x10316f49b*/
        goto LABEL_84; /*0x10316f49b*/
      v34 = 0; /*0x10316f4af*/
      do /*0x10316f4e9*/
      {
        v35 = *(unsigned __int8 *)(*v33 + v34); /*0x10316f4b6*/
        if ( v35 > 0x1F ) /*0x10316f4be*/
        {
          v35 = (char)v35; /*0x10316f4c8*/
          v36 = a2; /*0x10316f4cb*/
          v37 = "%c"; /*0x10316f4ce*/
        }
        else
        {
          v36 = a2; /*0x10316f4c0*/
          v37 = "\\x%02X"; /*0x10316f4c3*/
        }
        Luau::formatAppend(a1: v36, a2: (_DWORD)v37, a3: v35, a4: v12, a5, a6); /*0x10316f4d3*/
        v38 = v33[1]; /*0x10316f4d8*/
        if ( v34 > 0x1E ) /*0x10316f4e1*/
          break; /*0x10316f4e1*/
        ++v34; /*0x10316f4e3*/
      }
      while ( v34 < v38 ); /*0x10316f4e9*/
      if ( v38 > 0x1F ) /*0x10316f4ef*/
        v13 = "'..."; /*0x10316f4f5*/
      else
LABEL_84:
        v13 = "'"; /*0x10316f962*/
      goto LABEL_97; /*0x10316f4fc*/
    case 7uLL:
      v16 = *(_DWORD *)(v12 + 8); /*0x10316f2ec*/
      v17 = -1; /*0x10316f2ff*/
      if ( (v16 & 0x80000000) != 0 ) /*0x10316f305*/
        v17 = (v16 >> 10) & 0x3FF; /*0x10316f305*/
      if ( v16 < 0x40000000 ) /*0x10316f310*/
        return; /*0x10316f310*/
      v18 = 16LL * (unsigned int)(*(_DWORD *)(v7 + 40LL * ((v16 >> 20) & 0x3FF) + 8) - 1); /*0x10316f33a*/
      Luau::formatAppend( /*0x10316f352*/
        a1: a2,
        a2: (unsigned int)"%.*s",
        a3: *(_DWORD *)(a1[75] + v18 + 8),
        a4: *(_QWORD *)(a1[75] + v18),
        a5,
        a6);
      if ( v16 >> 30 == 1 ) /*0x10316f35b*/
        return; /*0x10316f35b*/
      v21 = 16LL * (unsigned int)(*(_DWORD *)(a1[12] + 40LL * v17 + 8) - 1); /*0x10316f379*/
      Luau::formatAppend( /*0x10316f391*/
        a1: a2,
        a2: (unsigned int)".%.*s",
        a3: *(_DWORD *)(a1[75] + v21 + 8),
        a4: *(_QWORD *)(a1[75] + v21),
        a5: v19,
        a6: v20);
      if ( v16 >> 30 != 3 ) /*0x10316f39a*/
        return; /*0x10316f39a*/
      v22 = a1[75]; /*0x10316f3af*/
      v23 = 16LL * (unsigned int)(*(_DWORD *)(a1[12] + 40LL * (v16 & 0x3FF) + 8) - 1); /*0x10316f3bc*/
      LODWORD(v24) = *(_DWORD *)(v22 + v23 + 8); /*0x10316f3c0*/
      v25 = *(_BYTE **)(v22 + v23); /*0x10316f3c4*/
      v26 = ".%.*s"; /*0x10316f3c8*/
      v27 = a2; /*0x10316f3cf*/
      goto LABEL_100; /*0x10316f3d2*/
    case 8uLL:
      if ( a4 == 0 ) /*0x10316f672*/
      {
        v13 = "{...}"; /*0x10316f959*/
        goto LABEL_97; /*0x10316f960*/
      }
      v54 = a1[21] + 264LL * *(unsigned int *)(v12 + 8); /*0x10316f68d*/
      v55 = *(_DWORD *)(v54 + 256); /*0x10316f691*/
      if ( v55 != 0 ) /*0x10316f69c*/
      {
        v56 = 0; /*0x10316f69e*/
        if ( v55 != 1 ) /*0x10316f6a4*/
        {
          v56 = 0; /*0x10316f6a6*/
          do /*0x10316f6b4*/
            v57 = 2 << v56++; /*0x10316f6ad*/
          while ( v57 < v55 ); /*0x10316f6b4*/
        }
        v55 = 1 << v56; /*0x10316f6bc*/
      }
      v58 = v55 - 1; /*0x10316f6c4*/
      if ( v55 == 0 ) /*0x10316f6c8*/
        v58 = 0; /*0x10316f6c8*/
      memset(v83, 0, 24); /*0x10316f6d3*/
      v59 = *(unsigned int *)(v54 + 256); /*0x10316f6de*/
      LODWORD(v84[0]) = 0; /*0x10316f6ea*/
      std::vector<unsigned int>::resize(a1: v83, a2: v59, a3: v84); /*0x10316f6f0*/
      memset(v84, 0, 24); /*0x10316f6fc*/
      v85 = v55; /*0x10316f707*/
      v82 = -1; /*0x10316f712*/
      std::vector<unsigned int>::resize(a1: v84, a2: (unsigned int)v55, a3: &v82); /*0x10316f718*/
      if ( *(_DWORD *)(v54 + 256) != 0 ) /*0x10316f726*/
      {
        v64 = a1[12]; /*0x10316f72c*/
        v61 = a1[75]; /*0x10316f730*/
        v60 = v83[0]; /*0x10316f737*/
        v65 = v84[0]; /*0x10316f73b*/
        v66 = 0; /*0x10316f73f*/
        do /*0x10316f7ae*/
        {
          v63 = *(_QWORD *)(v61 + 16LL * (unsigned int)(*(_DWORD *)(v64 + 40LL * *(int *)(v54 + 4 * v66) + 8) - 1) + 8); /*0x10316f755*/
          v67 = 0; /*0x10316f75a*/
          if ( v63 != 0 ) /*0x10316f763*/
          {
            v67 = *(_QWORD *)(v61 + 16LL * (unsigned int)(*(_DWORD *)(v64 + 40LL * *(int *)(v54 + 4 * v66) + 8) - 1) + 8); /*0x10316f769*/
            do /*0x10316f78c*/
              v67 ^= 32 * v67 /*0x10316f786*/
                   + (v67 >> 2)
                   + *(unsigned __int8 *)(*(_QWORD *)(v61
                                                    + 16LL
                                                    * (unsigned int)(*(_DWORD *)(v64 + 40LL * *(int *)(v54 + 4 * v66) + 8)
                                                                   - 1))
                                        + v63--
                                        - 1);
            while ( v63 != 0 ); /*0x10316f78c*/
          }
          v68 = v58 & v67; /*0x10316f78e*/
          v60[v66] = v68; /*0x10316f791*/
          if ( v65[v68] == -1 ) /*0x10316f79a*/
            v65[v68] = v66; /*0x10316f79c*/
          ++v66; /*0x10316f7a0*/
          v62 = *(unsigned int *)(v54 + 256); /*0x10316f7a3*/
        }
        while ( v66 < v62 ); /*0x10316f7ae*/
      }
      Luau::formatAppend(a1: a2, a2: (unsigned int)"{", a3: (_DWORD)v60, a4: v61, a5: v62, a6: v63); /*0x10316f7bc*/
      if ( *(_DWORD *)(v54 + 256) != 0 ) /*0x10316f7ca*/
      {
        v73 = 0; /*0x10316f7d7*/
        do /*0x10316f8a9*/
        {
          if ( v73 != 0 ) /*0x10316f7dd*/
            Luau::formatAppend(a1: a2, a2: (unsigned int)", ", a3: v69, a4: v70, a5: v71, a6: v72); /*0x10316f7eb*/
          Luau::formatAppend(a1: a2, a2: (unsigned int)"[", a3: v69, a4: v70, a5: v71, a6: v72); /*0x10316f7fc*/
          (*(void (__fastcall **)(_QWORD *, __int64, _QWORD, _QWORD))(*a1 + 48LL))( /*0x10316f810*/
            a1,
            a2,
            a3: *(unsigned int *)(v54 + 4 * v73),
            a4: 0);
          Luau::formatAppend(a1: a2, a2: (unsigned int)"]", a3: v74, a4: v75, a5: v76, a6: v77); /*0x10316f81f*/
          if ( *(_BYTE *)(v54 + 260) == 1 && *(_DWORD *)(v54 + 4 * v73 + 128) != -1 ) /*0x10316f838*/
          {
            Luau::formatAppend(a1: a2, a2: (unsigned int)" = ", a3: v78, a4: v79, a5: v80, a6: v81); /*0x10316f846*/
            (*(void (__fastcall **)(_QWORD *, __int64, _QWORD, _QWORD))(*a1 + 48LL))( /*0x10316f85e*/
              a1,
              a2,
              a3: *(unsigned int *)(v54 + 4 * v73 + 128),
              a4: 0);
          }
          Luau::formatAppend(a1: a2, a2: (unsigned int)" #%u", a3: *((_DWORD *)v83[0] + v73), a4: v79, a5: v80, a6: v81); /*0x10316f871*/
          v70 = (int)v84[0]; /*0x10316f87e*/
          if ( v73 != *((_DWORD *)v84[0] + *((unsigned int *)v83[0] + v73)) ) /*0x10316f888*/
            Luau::formatAppend(a1: a2, a2: (unsigned int)" (conflict)", a3: v69, a4: v84[0], a5: v71, a6: v72); /*0x10316f896*/
          ++v73; /*0x10316f89b*/
        }
        while ( v73 < *(unsigned int *)(v54 + 256) ); /*0x10316f8a9*/
      }
      Luau::formatAppend(a1: a2, a2: (unsigned int)"} sizenode=%u", a3: v85, a4: v70, a5: v71, a6: v72); /*0x10316f8be*/
      if ( v84[0] != nullptr ) /*0x10316f8ca*/
      {
        v84[1] = v84[0]; /*0x10316f8cc*/
        operator delete(a1: v84[0]); /*0x10316f8d0*/
      }
      if ( v83[0] != nullptr ) /*0x10316f8dc*/
      {
        v83[1] = v83[0]; /*0x10316f8e2*/
        operator delete(a1: v83[0]); /*0x10316f8e6*/
      }
      return; /*0x10316f8eb*/
    case 9uLL:
      v14 = *(unsigned int *)(v12 + 8); /*0x10316f27c*/
      v12 = a1[1]; /*0x10316f27f*/
      v9 = (const char *)(v12 + 136 * v14); /*0x10316f28e*/
      v15 = v9[64]; /*0x10316f292*/
      if ( a4 != 0 ) /*0x10316f299*/
      {
        if ( (v15 & 1) != 0 ) /*0x10316f2a1*/
        {
          if ( *((_QWORD *)v9 + 9) != 0 ) /*0x10316f970*/
          {
            v9 = *((const char **)v9 + 10); /*0x10316f972*/
            goto LABEL_87; /*0x10316f972*/
          }
        }
        else if ( v15 != 0 ) /*0x10316f2a9*/
        {
          LODWORD(v9) = (_DWORD)v9 + 65; /*0x10316f2af*/
LABEL_87:
          v13 = "function %s"; /*0x10316f976*/
          goto LABEL_97; /*0x10316f97d*/
        }
        v13 = "function"; /*0x10316f9b8*/
      }
      else
      {
        if ( (v15 & 1) != 0 ) /*0x10316f8f2*/
        {
          if ( *((_QWORD *)v9 + 9) == 0 ) /*0x10316f984*/
            return; /*0x10316f984*/
          v9 = *((const char **)v9 + 10); /*0x10316f986*/
        }
        else
        {
          if ( v15 == 0 ) /*0x10316f8fa*/
            return; /*0x10316f8fa*/
          LODWORD(v9) = (_DWORD)v9 + 65; /*0x10316f900*/
        }
        v13 = "'%s'"; /*0x10316f98a*/
      }
LABEL_97:
      Luau::formatAppend(a1: a2, a2: (_DWORD)v13, a3: (_DWORD)v9, a4: v12, a5, a6); /*0x10316f9ef*/
      return; /*0x10316f2c9*/
    case 0xAuLL:
      v39 = 56LL * *(unsigned int *)(v12 + 8); /*0x10316f504*/
      v40 = a1[24]; /*0x10316f508*/
      v41 = 16LL * (unsigned int)(*(_DWORD *)(v7 + 40LL * *(int *)(v40 + v39) + 8) - 1); /*0x10316f525*/
      Luau::formatAppend(
        a1: a2,
        a2: (unsigned int)"class %.*s (props: %zu, methods: %zu)",
        a3: *(_DWORD *)(a1[75] + v41 + 8),
        a4: *(_QWORD *)(a1[75] + v41),
        a5: (__int64)(*(_QWORD *)(v40 + v39 + 16) - *(_QWORD *)(v40 + v39 + 8)) >> 2,
        a6: (__int64)(*(_QWORD *)(v40 + v39 + 40) - *(_QWORD *)(v40 + v39 + 32)) >> 2);
      if ( a4 != 0 ) /*0x10316f561*/
      {
        v46 = (_QWORD *)(v39 + v40); /*0x10316f567*/
        if ( v46[1] != v46[2] ) /*0x10316f572*/
        {
          Luau::formatAppend(a1: a2, a2: (unsigned int)"\n  props:", a3: v42, a4: v43, a5: v44, a6: v45); /*0x10316f580*/
          v47 = v46[1]; /*0x10316f585*/
          if ( v46[2] != v47 ) /*0x10316f58d*/
          {
            v48 = 0; /*0x10316f596*/
            do /*0x10316f5e4*/
            {
              Luau::formatAppend( /*0x10316f5a9*/
                a1: a2,
                a2: (unsigned int)"\n    K%d [",
                a3: *(_DWORD *)(v47 + 4 * v48),
                a4: v43,
                a5: v44,
                a6: v45);
              (*(void (__fastcall **)(_QWORD *, __int64, _QWORD, _QWORD))(*a1 + 48LL))( /*0x10316f5c1*/
                a1,
                a2,
                a3: *(unsigned int *)(v46[1] + 4 * v48),
                a4: 0);
              std::string::append(a1: a2, a2: "]"); /*0x10316f5ca*/
              ++v48; /*0x10316f5cf*/
              v47 = v46[1]; /*0x10316f5d2*/
              v43 = (v46[2] - v47) >> 2; /*0x10316f5dd*/
            }
            while ( v48 < v43 ); /*0x10316f5e4*/
          }
        }
        if ( v46[4] != v46[5] ) /*0x10316f5ee*/
        {
          Luau::formatAppend(a1: a2, a2: (unsigned int)"\n  methods:", a3: v42, a4: v43, a5: v44, a6: v45); /*0x10316f600*/
          v52 = v46[4]; /*0x10316f605*/
          if ( v46[5] != v52 ) /*0x10316f60d*/
          {
            v53 = 0; /*0x10316f61a*/
            do /*0x10316f668*/
            {
              Luau::formatAppend( /*0x10316f62d*/
                a1: a2,
                a2: (unsigned int)"\n    K%d [",
                a3: *(_DWORD *)(v52 + 4 * v53),
                a4: v49,
                a5: v50,
                a6: v51);
              (*(void (__fastcall **)(_QWORD *, __int64, _QWORD, _QWORD))(*a1 + 48LL))( /*0x10316f645*/
                a1,
                a2,
                a3: *(unsigned int *)(v46[4] + 4 * v53),
                a4: 0);
              std::string::append(a1: a2, a2: "]"); /*0x10316f64e*/
              ++v53; /*0x10316f653*/
              v52 = v46[4]; /*0x10316f656*/
              v49 = (v46[5] - v52) >> 2; /*0x10316f661*/
            }
            while ( v53 < v49 ); /*0x10316f668*/
          }
        }
      }
      return; /*0x10316f668*/
    default:
      return;
  }
}

// Address: 0x10316faa4 | Name: __ZNK4Luau15BytecodeBuilder15dumpInstructionEPKjRNSt3__112basic_stringIcNS3_11char_traitsIcEENS3_9allocatorIcEEEEi
__int64 __fastcall Luau::BytecodeBuilder::dumpInstruction(
        __int64 *a1,
        unsigned int *a2,
        __int64 a3,
        unsigned int a4,
        unsigned int a5,
        const char *a6)
{
  __int64 result; // rax
  unsigned int v7; // ebx
  int v8; // ecx
  const char *v12; // rsi
  int v13; // edx
  const char *v14; // rsi
  int v15; // edx
  int v16; // ecx
  unsigned int v17; // ebx
  const char *v18; // rsi
  int v19; // edx
  int v20; // ebx
  int v21; // r8d
  const char *v22; // rsi
  int v23; // edx
  int v24; // ecx
  const char *v25; // r9
  const char *v26; // rsi
  int v27; // edx
  const char *v28; // rsi
  int v29; // edi
  unsigned int v30; // ecx
  int v31; // edi
  int v32; // ebx
  int v33; // ebx
  int v34; // ebx
  unsigned int v35; // ebx
  unsigned int v36; // ebx
  int v37; // edx
  const char *v38; // rsi
  int v39; // edx
  unsigned int v40; // r12d
  const char *v41; // rsi
  const char *v42; // r8
  __int64 v43; // rdx
  int v44; // ebx
  int v45; // eax
  __int64 v46; // rax
  __int64 *v47; // rdi
  __int64 v48; // rsi
  const char *v49; // rdx
  int v50; // ecx
  const char *v51; // rdx

  result = a4; /*0x10316fab3*/
  v7 = *a2; /*0x10316fab5*/
  v8 = (unsigned __int8)*a2 - 1; /*0x10316faba*/
  switch ( (unsigned __int8)*a2 ) /*0x10316fadc*/
  {
    case 1u: /*0x10316fadc*/
      v12 = "BREAK\n"; /*0x10316fade*/
      return Luau::formatAppend(a1: a3, a2: (_DWORD)v12, a3, a4: v8, a5, (_DWORD)a6); /*0x10316fae5*/
    case 2u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316ffbf*/
      v14 = "LOADNIL R%d\n"; /*0x10316ffc2*/
      goto LABEL_55; /*0x10316ffc2*/
    case 3u: /*0x10316fadc*/
      if ( v7 >= 0x1000000 ) /*0x10316fefd*/
      {
        v13 = BYTE1(v7); /*0x103170686*/
        v14 = "LOADB R%d %d +%d\n"; /*0x10317068d*/
        v31 = a3; /*0x103170694*/
        a5 = HIBYTE(v7); /*0x103170697*/
        v8 = BYTE2(v7); /*0x10317069a*/
      }
      else
      {
        v13 = BYTE1(v7); /*0x10316ff03*/
        v32 = HIWORD(v7); /*0x10316ff06*/
        v14 = "LOADB R%d %d\n"; /*0x10316ff09*/
LABEL_119:
        v31 = a3; /*0x103170572*/
        v8 = v32; /*0x103170575*/
      }
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x103170575*/
    case 4u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317009e*/
      v32 = (int)v7 >> 16; /*0x1031700a1*/
      v14 = "LOADN R%d %d\n"; /*0x1031700a4*/
      goto LABEL_119; /*0x1031700ab*/
    case 5u: /*0x10316fadc*/
      v37 = BYTE1(v7); /*0x1031700d2*/
      v17 = (int)v7 >> 16; /*0x1031700d5*/
      v38 = "LOADK R%d K%d ["; /*0x1031700d8*/
      goto LABEL_85; /*0x1031700df*/
    case 6u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316ff47*/
      v32 = BYTE2(v7); /*0x10316ff4c*/
      v14 = "MOVE R%d R%d\n"; /*0x10316ff4e*/
      goto LABEL_119; /*0x10316ff55*/
    case 7u: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x10316ffd1*/
      v30 = a2[1]; /*0x10316ffd4*/
      v28 = "GETGLOBAL R%d K%d ["; /*0x10316ffd9*/
      goto LABEL_87; /*0x10316ffe0*/
    case 8u: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x10316ffe5*/
      v30 = a2[1]; /*0x10316ffe8*/
      v28 = "SETGLOBAL R%d K%d ["; /*0x10316ffed*/
      goto LABEL_87; /*0x10316fff4*/
    case 9u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x1031701cb*/
      v32 = BYTE2(v7); /*0x1031701d0*/
      v14 = "GETUPVAL R%d %d\n"; /*0x1031701d2*/
      goto LABEL_119; /*0x1031701d9*/
    case 0xAu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170110*/
      v32 = BYTE2(v7); /*0x103170115*/
      v14 = "SETUPVAL R%d %d\n"; /*0x103170117*/
      goto LABEL_119; /*0x10317011e*/
    case 0xBu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fde1*/
      v14 = "CLOSEUPVALS R%d\n"; /*0x10316fde4*/
LABEL_55:
      v31 = a3; /*0x10316ffc9*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x10316ffcc*/
    case 0xCu: /*0x10316fadc*/
      v37 = BYTE1(v7); /*0x10316ff5a*/
      v17 = (int)v7 >> 16; /*0x10316ff5d*/
      v38 = "GETIMPORT R%d %d ["; /*0x10316ff60*/
      goto LABEL_85; /*0x10316ff67*/
    case 0xDu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316febd*/
      v8 = BYTE2(v7); /*0x10316fec4*/
      v34 = HIBYTE(v7); /*0x10316fec6*/
      v14 = "GETTABLE R%d R%d R%d\n"; /*0x10316fec9*/
      goto LABEL_103; /*0x10316fed0*/
    case 0xEu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fda2*/
      v8 = BYTE2(v7); /*0x10316fda9*/
      v34 = HIBYTE(v7); /*0x10316fdab*/
      v14 = "SETTABLE R%d R%d R%d\n"; /*0x10316fdae*/
      goto LABEL_103; /*0x10316fdb5*/
    case 0xFu: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x103170025*/
      v36 = BYTE2(v7); /*0x10317002a*/
      a5 = a2[1]; /*0x10317002c*/
      v28 = "GETTABLEKS R%d R%d K%d ["; /*0x103170031*/
      goto LABEL_61; /*0x103170038*/
    case 0x10u: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x103170044*/
      v36 = BYTE2(v7); /*0x103170049*/
      a5 = a2[1]; /*0x10317004b*/
      v28 = "SETTABLEKS R%d R%d K%d ["; /*0x103170050*/
      goto LABEL_61; /*0x103170050*/
    case 0x11u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317022e*/
      v8 = BYTE2(v7); /*0x103170235*/
      v34 = HIBYTE(v7) + 1; /*0x10317023a*/
      v14 = "GETTABLEN R%d R%d %d\n"; /*0x10317023c*/
      goto LABEL_103; /*0x103170243*/
    case 0x12u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x1031701e8*/
      v8 = BYTE2(v7); /*0x1031701ef*/
      v34 = HIBYTE(v7) + 1; /*0x1031701f4*/
      v14 = "SETTABLEN R%d R%d %d\n"; /*0x1031701f6*/
      goto LABEL_103; /*0x1031701fd*/
    case 0x13u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fd67*/
      v32 = (int)v7 >> 16; /*0x10316fd6a*/
      v14 = "NEWCLOSURE R%d P%d\n"; /*0x10316fd6d*/
      goto LABEL_119; /*0x10316fd74*/
    case 0x14u: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x10316fedf*/
      v36 = BYTE2(v7); /*0x10316fee4*/
      a5 = a2[1]; /*0x10316fee6*/
      v28 = "NAMECALL R%d R%d K%d ["; /*0x10316feeb*/
LABEL_61:
      v29 = a3; /*0x103170057*/
      v30 = v36; /*0x10317005a*/
      goto LABEL_128; /*0x10317005e*/
    case 0x15u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170283*/
      v8 = BYTE2(v7) - 1; /*0x10317028c*/
      v34 = HIBYTE(v7) - 1; /*0x103170291*/
      v14 = "CALL R%d %d %d\n"; /*0x103170293*/
      goto LABEL_103; /*0x10317029a*/
    case 0x16u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fd83*/
      v32 = BYTE2(v7) - 1; /*0x10316fd8a*/
      v14 = "RETURN R%d %d\n"; /*0x10316fd8c*/
      goto LABEL_119; /*0x10316fd93*/
    case 0x17u: /*0x10316fadc*/
      v14 = "JUMP L%d\n"; /*0x10316fea2*/
      goto LABEL_42; /*0x10316fea2*/
    case 0x18u: /*0x10316fadc*/
      v14 = "JUMPBACK L%d\n"; /*0x10316fdd5*/
      goto LABEL_42; /*0x10316fddc*/
    case 0x19u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316ff8e*/
      v14 = "JUMPIF R%d L%d\n"; /*0x10316ff91*/
      goto LABEL_111; /*0x10316ff98*/
    case 0x1Au: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fdba*/
      v14 = "JUMPIFNOT R%d L%d\n"; /*0x10316fdbd*/
      goto LABEL_111; /*0x10316fdc4*/
    case 0x1Bu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317037c*/
      v8 = a2[1]; /*0x10317037f*/
      v14 = "JUMPIFEQ R%d R%d L%d\n"; /*0x103170384*/
      goto LABEL_95; /*0x103170384*/
    case 0x1Cu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317036b*/
      v8 = a2[1]; /*0x10317036e*/
      v14 = "JUMPIFLE R%d R%d L%d\n"; /*0x103170373*/
      goto LABEL_95; /*0x10317037a*/
    case 0x1Du: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fe6c*/
      v8 = a2[1]; /*0x10316fe6f*/
      v14 = "JUMPIFLT R%d R%d L%d\n"; /*0x10316fe74*/
      goto LABEL_95; /*0x10316fe7b*/
    case 0x1Eu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316ff15*/
      v8 = a2[1]; /*0x10316ff18*/
      v14 = "JUMPIFNOTEQ R%d R%d L%d\n"; /*0x10316ff1d*/
      goto LABEL_95; /*0x10316ff24*/
    case 0x1Fu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317008a*/
      v8 = a2[1]; /*0x10317008d*/
      v14 = "JUMPIFNOTLE R%d R%d L%d\n"; /*0x103170092*/
      goto LABEL_95; /*0x103170099*/
    case 0x20u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316ff29*/
      v8 = a2[1]; /*0x10316ff2c*/
      v14 = "JUMPIFNOTLT R%d R%d L%d\n"; /*0x10316ff31*/
      goto LABEL_95; /*0x10316ff38*/
    case 0x21u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170406*/
      v8 = BYTE2(v7); /*0x10317040d*/
      v34 = HIBYTE(v7); /*0x10317040f*/
      v14 = "ADD R%d R%d R%d\n"; /*0x103170412*/
      goto LABEL_103; /*0x103170412*/
    case 0x22u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fdfa*/
      v8 = BYTE2(v7); /*0x10316fe01*/
      v34 = HIBYTE(v7); /*0x10316fe03*/
      v14 = "SUB R%d R%d R%d\n"; /*0x10316fe06*/
      goto LABEL_103; /*0x10316fe0d*/
    case 0x23u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317020c*/
      v8 = BYTE2(v7); /*0x103170213*/
      v34 = HIBYTE(v7); /*0x103170215*/
      v14 = "MUL R%d R%d R%d\n"; /*0x103170218*/
      goto LABEL_103; /*0x10317021f*/
    case 0x24u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x1031703e7*/
      v8 = BYTE2(v7); /*0x1031703ee*/
      v34 = HIBYTE(v7); /*0x1031703f0*/
      v14 = "DIV R%d R%d R%d\n"; /*0x1031703f3*/
      goto LABEL_103; /*0x1031703fa*/
    case 0x25u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fd1e*/
      v8 = BYTE2(v7); /*0x10316fd25*/
      v34 = HIBYTE(v7); /*0x10316fd27*/
      v14 = "MOD R%d R%d R%d\n"; /*0x10316fd2a*/
      goto LABEL_103; /*0x10316fd31*/
    case 0x26u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x1031700ee*/
      v8 = BYTE2(v7); /*0x1031700f5*/
      v34 = HIBYTE(v7); /*0x1031700f7*/
      v14 = "POW R%d R%d R%d\n"; /*0x1031700fa*/
      goto LABEL_103; /*0x103170101*/
    case 0x27u: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10316fcfc*/
      v16 = BYTE2(v7); /*0x10316fd03*/
      v17 = HIBYTE(v7); /*0x10316fd05*/
      v18 = "ADDK R%d R%d K%d ["; /*0x10316fd08*/
      goto LABEL_105; /*0x10316fd0f*/
    case 0x28u: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10316fcb7*/
      v16 = BYTE2(v7); /*0x10316fcbe*/
      v17 = HIBYTE(v7); /*0x10316fcc0*/
      v18 = "SUBK R%d R%d K%d ["; /*0x10316fcc3*/
      goto LABEL_105; /*0x10316fcca*/
    case 0x29u: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10316fe8a*/
      v16 = BYTE2(v7); /*0x10316fe91*/
      v17 = HIBYTE(v7); /*0x10316fe93*/
      v18 = "MULK R%d R%d K%d ["; /*0x10316fe96*/
      goto LABEL_105; /*0x10316fe9d*/
    case 0x2Au: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10316ffa7*/
      v16 = BYTE2(v7); /*0x10316ffae*/
      v17 = HIBYTE(v7); /*0x10316ffb0*/
      v18 = "DIVK R%d R%d K%d ["; /*0x10316ffb3*/
      goto LABEL_105; /*0x10316ffba*/
    case 0x2Bu: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x103170261*/
      v16 = BYTE2(v7); /*0x103170268*/
      v17 = HIBYTE(v7); /*0x10317026a*/
      v18 = "MODK R%d R%d K%d ["; /*0x10317026d*/
      goto LABEL_105; /*0x103170274*/
    case 0x2Cu: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10316fc0f*/
      v16 = BYTE2(v7); /*0x10316fc16*/
      v17 = HIBYTE(v7); /*0x10316fc18*/
      v18 = "POWK R%d R%d K%d ["; /*0x10316fc1b*/
      goto LABEL_105; /*0x10316fc22*/
    case 0x2Du: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fd40*/
      v8 = BYTE2(v7); /*0x10316fd47*/
      v34 = HIBYTE(v7); /*0x10316fd49*/
      v14 = "AND R%d R%d R%d\n"; /*0x10316fd4c*/
      goto LABEL_103; /*0x10316fd53*/
    case 0x2Eu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170003*/
      v8 = BYTE2(v7); /*0x10317000a*/
      v34 = HIBYTE(v7); /*0x10317000c*/
      v14 = "OR R%d R%d R%d\n"; /*0x10317000f*/
      goto LABEL_103; /*0x103170016*/
    case 0x2Fu: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x1031700ba*/
      v16 = BYTE2(v7); /*0x1031700c1*/
      v17 = HIBYTE(v7); /*0x1031700c3*/
      v18 = "ANDK R%d R%d K%d ["; /*0x1031700c6*/
      goto LABEL_105; /*0x1031700cd*/
    case 0x30u: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10317043c*/
      v16 = BYTE2(v7); /*0x103170443*/
      v17 = HIBYTE(v7); /*0x103170445*/
      v18 = "ORK R%d R%d K%d ["; /*0x103170448*/
      goto LABEL_105; /*0x103170448*/
    case 0x31u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170072*/
      v8 = BYTE2(v7); /*0x103170079*/
      v34 = HIBYTE(v7); /*0x10317007b*/
      v14 = "CONCAT R%d R%d R%d\n"; /*0x10317007e*/
      goto LABEL_103; /*0x103170085*/
    case 0x32u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fc6f*/
      v32 = BYTE2(v7); /*0x10316fc74*/
      v14 = "NOT R%d R%d\n"; /*0x10316fc76*/
      goto LABEL_119; /*0x10316fc7d*/
    case 0x33u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fc31*/
      v32 = BYTE2(v7); /*0x10316fc36*/
      v14 = "MINUS R%d R%d\n"; /*0x10316fc38*/
      goto LABEL_119; /*0x10316fc3f*/
    case 0x34u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170548*/
      v32 = BYTE2(v7); /*0x10317054d*/
      v14 = "LENGTH R%d R%d\n"; /*0x10317054f*/
      goto LABEL_119; /*0x103170556*/
    case 0x35u: /*0x10316fadc*/
      v45 = 1 << (BYTE2(v7) - 1); /*0x1031703b9*/
      if ( BYTE2(v7) == 0 ) /*0x1031703c0*/
        v45 = 0; /*0x1031703c0*/
      a5 = a2[1]; /*0x1031703c8*/
      v14 = "NEWTABLE R%d %d %d\n"; /*0x1031703cd*/
      v31 = a3; /*0x1031703d4*/
      v13 = BYTE1(v7); /*0x1031703d7*/
      v8 = v45; /*0x1031703d9*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x1031703db*/
    case 0x36u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fc53*/
      v32 = (int)v7 >> 16; /*0x10316fc56*/
      v14 = "DUPTABLE R%d %d\n"; /*0x10316fc59*/
      goto LABEL_119; /*0x10316fc60*/
    case 0x37u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317012d*/
      v8 = BYTE2(v7); /*0x103170134*/
      v33 = HIBYTE(v7) - 1; /*0x103170139*/
      LODWORD(a6) = a2[1]; /*0x10317013b*/
      v14 = "SETLIST R%d R%d %d [%d]\n"; /*0x103170140*/
      goto LABEL_70; /*0x103170140*/
    case 0x38u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170248*/
      v14 = "FORNPREP R%d L%d\n"; /*0x10317024b*/
      goto LABEL_111; /*0x103170252*/
    case 0x39u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fc44*/
      v14 = "FORNLOOP R%d L%d\n"; /*0x10316fc47*/
      goto LABEL_111; /*0x10316fc4e*/
    case 0x3Au: /*0x10316fadc*/
      v44 = BYTE1(v7); /*0x10317032b*/
      a5 = (unsigned __int8)a2[1]; /*0x103170333*/
      a6 = ""; /*0x10317033f*/
      if ( (a2[1] & 0x80000000) != 0 ) /*0x103170346*/
        a6 = " [inext]"; /*0x103170346*/
      v14 = "FORGLOOP R%d L%d %d%s\n"; /*0x10317034a*/
      v31 = a3; /*0x103170351*/
      v13 = v44; /*0x103170354*/
      v8 = result; /*0x103170356*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x103170366*/
    case 0x3Bu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10317029f*/
      v14 = "FORGPREP_INEXT R%d L%d\n"; /*0x1031702a2*/
      goto LABEL_111; /*0x1031702a9*/
    case 0x3Cu: /*0x10316fadc*/
      return Luau::formatAppend( /*0x1031705f1*/
               a1: a3,
               a2: (unsigned int)"FASTCALL3 %d R%d R%d R%d L%d\n",
               a3: BYTE1(v7),
               a4: BYTE2(v7),
               a5: (unsigned __int8)a2[1],
               a6: (unsigned __int8)BYTE1(a2[1]));
    case 0x3Du: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170496*/
      v14 = "FORGPREP_NEXT R%d L%d\n"; /*0x103170499*/
      goto LABEL_111; /*0x103170499*/
    case 0x3Fu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170562*/
      v32 = BYTE2(v7) - 1; /*0x103170569*/
      v14 = "GETVARARGS R%d %d\n"; /*0x10317056b*/
      goto LABEL_119; /*0x10317056b*/
    case 0x40u: /*0x10316fadc*/
      v37 = BYTE1(v7); /*0x1031702ae*/
      v17 = (int)v7 >> 16; /*0x1031702b1*/
      v38 = "DUPCLOSURE R%d K%d ["; /*0x1031702b4*/
LABEL_85:
      Luau::formatAppend(a1: a3, a2: (_DWORD)v38, a3: v37, a4: v17, a5, (_DWORD)a6); /*0x1031702bb*/
      goto LABEL_106; /*0x1031702c7*/
    case 0x42u: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x1031702cc*/
      v30 = a2[1]; /*0x1031702cf*/
      v28 = "LOADKX R%d K%d ["; /*0x1031702d4*/
LABEL_87:
      v29 = a3; /*0x1031702db*/
      goto LABEL_128; /*0x1031702db*/
    case 0x43u: /*0x10316fadc*/
      v14 = "JUMPX L%d\n"; /*0x10316fdc9*/
LABEL_42:
      v31 = a3; /*0x10316fea9*/
      v13 = result; /*0x10316feac*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x10316feae*/
    case 0x44u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170396*/
      v14 = "FASTCALL %d L%d\n"; /*0x103170399*/
      goto LABEL_111; /*0x1031703a0*/
    case 0x45u: /*0x10316fadc*/
      v12 = "COVERAGE\n"; /*0x10316fb74*/
      return Luau::formatAppend(a1: a3, a2: (_DWORD)v12, a3, a4: v8, a5, (_DWORD)a6); /*0x10316fb8c*/
    case 0x46u: /*0x10316fadc*/
      if ( BYTE1(v7) == 1 ) /*0x103170473*/
      {
        v49 = "REF"; /*0x1031706a1*/
      }
      else
      {
        if ( BYTE1(v7) == 2 ) /*0x10317047f*/
        {
          v49 = "UPVAL"; /*0x103170485*/
          v50 = 85; /*0x10317048c*/
          return Luau::formatAppend( /*0x103170491*/
                   a1: a3,
                   a2: (unsigned int)"CAPTURE %s %c%d\n",
                   a3: (_DWORD)v49,
                   a4: v50,
                   a5: BYTE2(v7),
                   (_DWORD)a6);
        }
        v49 = ""; /*0x1031706b7*/
        if ( (v7 & 0xFF00) == 0 ) /*0x1031706be*/
          v49 = "VAL"; /*0x1031706be*/
      }
      v50 = 82; /*0x1031706c2*/
      return Luau::formatAppend( /*0x10316fb80*/
               a1: a3,
               a2: (unsigned int)"CAPTURE %s %c%d\n",
               a3: (_DWORD)v49,
               a4: v50,
               a5: BYTE2(v7),
               (_DWORD)a6);
    case 0x47u: /*0x10316fadc*/
      v39 = BYTE1(v7); /*0x1031704b4*/
      v40 = BYTE2(v7); /*0x1031704bd*/
      v41 = "SUBRK R%d K%d ["; /*0x1031704c0*/
      goto LABEL_113; /*0x1031704c0*/
    case 0x48u: /*0x10316fadc*/
      v39 = BYTE1(v7); /*0x10316ff76*/
      v40 = BYTE2(v7); /*0x10316ff7f*/
      v41 = "DIVRK R%d K%d ["; /*0x10316ff82*/
LABEL_113:
      Luau::formatAppend(a1: a3, a2: (_DWORD)v41, a3: v39, a4: v40, a5, (_DWORD)a6); /*0x1031704c7*/
      (*(void (__fastcall **)(__int64 *, __int64, _QWORD, _QWORD))(*a1 + 48))(a1, a2: a3, a3: v40, a4: 0); /*0x1031704e2*/
      v14 = "] R%d\n"; /*0x1031704e8*/
      v31 = a3; /*0x1031704ef*/
      v13 = HIBYTE(v7); /*0x1031704f2*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x103170502*/
    case 0x49u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x103170594*/
      v14 = "FASTCALL1 %d R%d L%d\n"; /*0x10317059b*/
      v31 = a3; /*0x1031705a2*/
      v8 = BYTE2(v7); /*0x1031705a5*/
      goto LABEL_96; /*0x1031705a7*/
    case 0x4Au: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x1031702f4*/
      a5 = a2[1]; /*0x1031702fb*/
      v14 = "FASTCALL2 %d R%d R%d L%d\n"; /*0x103170300*/
      v31 = a3; /*0x103170307*/
      v8 = BYTE2(v7); /*0x10317030a*/
      LODWORD(a6) = result; /*0x10317030c*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x10317031d*/
    case 0x4Bu: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x10316fbb2*/
      a5 = a2[1]; /*0x10316fbb9*/
      v28 = "FASTCALL2K %d R%d K%d L%d ["; /*0x10316fbbe*/
      v30 = BYTE2(v7); /*0x10316fbc5*/
      v29 = a3; /*0x10316fbc7*/
      LODWORD(a6) = result; /*0x10316fbca*/
      goto LABEL_128; /*0x10316fbcd*/
    case 0x4Cu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fd58*/
      v14 = "FORGPREP R%d L%d\n"; /*0x10316fd5b*/
LABEL_111:
      v31 = a3; /*0x1031704a0*/
      v8 = result; /*0x1031704a3*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x1031704a5*/
    case 0x4Du: /*0x10316fadc*/
      v42 = " NOT"; /*0x10317015f*/
      if ( (a2[1] & 0x80000000) == 0 ) /*0x103170166*/
        v42 = ""; /*0x103170166*/
      return Luau::formatAppend( /*0x103170187*/
               a1: a3,
               a2: (unsigned int)"JUMPXEQKNIL R%d L%d%s\n",
               a3: BYTE1(v7),
               a4: result,
               a5: (_DWORD)v42,
               (_DWORD)a6);
    case 0x4Eu: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fbd2*/
      v8 = a2[1] & 1; /*0x10316fbdc*/
      a6 = " NOT"; /*0x10316fbe8*/
      if ( (a2[1] & 0x80000000) == 0 ) /*0x10316fbef*/
        a6 = ""; /*0x10316fbef*/
      v14 = "JUMPXEQKB R%d %d L%d%s\n"; /*0x10316fbf3*/
      v31 = a3; /*0x10316fbfa*/
      a5 = result; /*0x10316fbfd*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x10316fc00*/
    case 0x4Fu: /*0x10316fadc*/
      v23 = BYTE1(v7); /*0x1031705f2*/
      v24 = a2[1] & 0xFFFFFF; /*0x103170601*/
      v25 = " NOT"; /*0x10317060c*/
      if ( (a2[1] & 0x80000000) == 0 ) /*0x103170613*/
        v25 = ""; /*0x103170613*/
      v26 = "JUMPXEQKN R%d K%d L%d%s ["; /*0x103170617*/
      goto LABEL_126; /*0x103170617*/
    case 0x50u: /*0x10316fadc*/
      v23 = BYTE1(v7); /*0x10316fb43*/
      v24 = a2[1] & 0xFFFFFF; /*0x10316fb52*/
      v25 = " NOT"; /*0x10316fb5d*/
      if ( (a2[1] & 0x80000000) == 0 ) /*0x10316fb64*/
        v25 = ""; /*0x10316fb64*/
      v26 = "JUMPXEQKS R%d K%d L%d%s ["; /*0x10316fb68*/
LABEL_126:
      Luau::formatAppend(a1: a3, a2: (_DWORD)v26, a3: v23, a4: v24, a5: result, a6: (_DWORD)v25); /*0x10317061e*/
      v17 = a2[1] & 0xFFFFFF; /*0x10317062b*/
      goto LABEL_106; /*0x103170630*/
    case 0x51u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fe54*/
      v8 = BYTE2(v7); /*0x10316fe5b*/
      v34 = HIBYTE(v7); /*0x10316fe5d*/
      v14 = "IDIV R%d R%d R%d\n"; /*0x10316fe60*/
LABEL_103:
      v31 = a3; /*0x103170419*/
      a5 = v34; /*0x10317041c*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x10317042d*/
    case 0x52u: /*0x10316fadc*/
      v15 = BYTE1(v7); /*0x10316fb08*/
      v16 = BYTE2(v7); /*0x10316fb0f*/
      v17 = HIBYTE(v7); /*0x10316fb11*/
      v18 = "IDIVK R%d R%d K%d ["; /*0x10316fb14*/
LABEL_105:
      Luau::formatAppend(a1: a3, a2: (_DWORD)v18, a3: v15, a4: v16, a5: v17, (_DWORD)a6); /*0x10317044f*/
LABEL_106:
      v46 = *a1; /*0x10317045c*/
      v47 = a1; /*0x10317045f*/
      v48 = a3; /*0x103170462*/
      v43 = v17; /*0x103170465*/
      goto LABEL_130; /*0x103170467*/
    case 0x53u: /*0x10316fadc*/
      v19 = BYTE1(v7); /*0x10316fb2a*/
      v20 = BYTE2(v7); /*0x10316fb2f*/
      v21 = *((unsigned __int16 *)a2 + 2); /*0x10316fb31*/
      v22 = "GETUDATAKS R%d R%d K%d ["; /*0x10316fb37*/
      goto LABEL_75; /*0x10316fb3e*/
    case 0x54u: /*0x10316fadc*/
      v19 = BYTE1(v7); /*0x10316fcd9*/
      v20 = BYTE2(v7); /*0x10316fcde*/
      v21 = *((unsigned __int16 *)a2 + 2); /*0x10316fce0*/
      v22 = "SETUDATAKS R%d R%d K%d ["; /*0x10316fce6*/
      goto LABEL_75; /*0x10316fced*/
    case 0x55u: /*0x10316fadc*/
      v19 = BYTE1(v7); /*0x103170196*/
      v20 = BYTE2(v7); /*0x10317019b*/
      v21 = *((unsigned __int16 *)a2 + 2); /*0x10317019d*/
      v22 = "NAMECALLUDATA R%d R%d K%d ["; /*0x1031701a3*/
LABEL_75:
      Luau::formatAppend(a1: a3, a2: (_DWORD)v22, a3: v19, a4: v20, a5: v21, (_DWORD)a6); /*0x1031701aa*/
      v43 = *((unsigned __int16 *)a2 + 2); /*0x1031701b6*/
      goto LABEL_129; /*0x1031701bc*/
    case 0x56u: /*0x10316fadc*/
      v27 = BYTE1(v7); /*0x10316fb91*/
      v28 = "NEWCLASSMEMBER R%d R%d ["; /*0x10316fb97*/
      v29 = a3; /*0x10316fb9e*/
      v30 = HIBYTE(v7); /*0x10316fba1*/
      goto LABEL_128; /*0x10316fba3*/
    case 0x57u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316fc8c*/
      v8 = BYTE2(v7) - 1; /*0x10316fc95*/
      v33 = HIBYTE(v7) - 1; /*0x10316fc9a*/
      LODWORD(a6) = a2[1]; /*0x10316fc9c*/
      v14 = "CALLFB R%d %d %d [%d]\n"; /*0x10316fca1*/
LABEL_70:
      v31 = a3; /*0x103170147*/
      a5 = v33; /*0x10317014a*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x10317014d*/
    case 0x58u: /*0x10316fadc*/
      v13 = BYTE1(v7); /*0x10316faea*/
      v8 = a2[1]; /*0x10316faed*/
      v14 = "CMPPROTO R%d #%d L%d\n"; /*0x10316faf2*/
LABEL_95:
      v31 = a3; /*0x10317038b*/
LABEL_96:
      a5 = result; /*0x10317038e*/
      return Luau::formatAppend(a1: v31, a2: (_DWORD)v14, a3: v13, a4: v8, a5, (_DWORD)a6); /*0x103170585*/
    case 0x59u: /*0x10316fadc*/
      v51 = "xpcall"; /*0x103170514*/
      if ( (v7 & 0xFF00) == 0 ) /*0x10317051b*/
        v51 = "pcall"; /*0x10317051b*/
      return Luau::formatAppend( /*0x103170539*/
               a1: a3,
               a2: (unsigned int)"FASTPCALL %s L%d\n",
               a3: (_DWORD)v51,
               a4: result,
               a5,
               (_DWORD)a6);
    case 0x5Au: /*0x10316fadc*/
      v30 = BYTE2(v7); /*0x10316fe1c*/
      v27 = BYTE1(v7); /*0x10316fe23*/
      v35 = HIBYTE(v7); /*0x10316fe25*/
      a5 = a2[1]; /*0x10316fe28*/
      if ( v30 == 255 ) /*0x10316fe2f*/
      {
        v28 = "NEWCLASS R%d no_base K%d %d ["; /*0x10316fe35*/
        v29 = a3; /*0x10316fe3c*/
        v30 = a5; /*0x10316fe3f*/
        a5 = v35; /*0x10316fe42*/
      }
      else
      {
        v28 = "NEWCLASS R%d R%d K%d %d ["; /*0x103170635*/
        v29 = a3; /*0x10317063c*/
        LODWORD(a6) = v35; /*0x10317063f*/
      }
LABEL_128:
      Luau::formatAppend(a1: v29, a2: (_DWORD)v28, a3: v27, a4: v30, a5, (_DWORD)a6); /*0x103170642*/
      v43 = a2[1]; /*0x103170649*/
LABEL_129:
      v46 = *a1; /*0x10317064e*/
      v47 = a1; /*0x103170651*/
      v48 = a3; /*0x103170654*/
LABEL_130:
      (*(void (__fastcall **)(__int64 *, __int64, __int64, _QWORD))(v46 + 48))(a1: v47, a2: v48, a3: v43, a4: 0); /*0x103170657*/
      return std::string::append(a1: a3, a2: "]\n"); /*0x103170672*/
    default:
      return result;
  }
}

// Address: 0x103170854 | Name: __ZNK4Luau15BytecodeBuilder19dumpCurrentFunctionERNSt3__16vectorIiNS1_9allocatorIiEEEE
unsigned __int8 *__fastcall Luau::BytecodeBuilder::dumpCurrentFunction(
        unsigned __int8 *a1,
        __int64 *a2,
        _QWORD *a3,
        __int64 a4,
        int a5,
        int a6)
{
  unsigned __int8 *v6; // rbx
  int v7; // ecx
  __int64 v9; // rax
  __int64 v10; // r14
  unsigned __int64 v11; // r13
  __int64 v12; // r8
  int v13; // ecx
  int v14; // r9d
  __int64 v15; // r12
  unsigned __int8 i; // r14
  unsigned int v17; // eax
  unsigned __int64 v18; // rcx
  char *v19; // rcx
  char v20; // r13
  unsigned __int64 v21; // rax
  __int64 v22; // rdx
  unsigned __int64 v23; // rcx
  const char *v24; // r8
  __int64 v25; // rax
  unsigned __int64 v26; // r12
  int v27; // r13d
  unsigned __int64 v28; // rax
  __int64 v29; // rdx
  unsigned __int64 v30; // rcx
  const char *v31; // r8
  __int64 v32; // r14
  __int64 v33; // r12
  unsigned __int64 v34; // r13
  int v35; // ebx
  unsigned __int64 v36; // rax
  __int64 v37; // rdx
  unsigned __int64 v38; // rcx
  const char *v39; // r8
  int v40; // eax
  __int64 v41; // rcx
  unsigned __int64 v42; // r13
  int v43; // edx
  int v44; // ecx
  int v45; // r8d
  int v46; // r9d
  __int64 v47; // rsi
  unsigned int v48; // edx
  __int64 v49; // rax
  unsigned __int64 v50; // r14
  int JumpTarget; // eax
  unsigned __int64 v52; // rsi
  _DWORD *v53; // rax
  __int64 v54; // rcx
  __int64 v55; // rdx
  int v56; // edi
  int v57; // r8d
  int v58; // r9d
  __int64 v59; // r13
  __int64 v60; // rax
  unsigned __int64 v61; // r14
  unsigned __int64 v62; // r12
  unsigned int v63; // ecx
  __int64 v64; // rcx
  unsigned int v65; // esi
  int v66; // eax
  __int64 v67; // rax
  int v68; // r13d
  __int64 v69; // rcx
  unsigned int v70; // edx
  unsigned int *v71; // r13
  int v72; // eax
  unsigned int v73; // r8d
  const char *v74; // r9
  unsigned int v75; // ecx
  __int64 v76; // rcx
  unsigned int v77; // eax
  unsigned int v78; // eax
  __int64 v80; // [rsp+10h] [rbp-70h]
  void *v81; // [rsp+18h] [rbp-68h] BYREF
  _BYTE *v82; // [rsp+20h] [rbp-60h]
  int v83; // [rsp+30h] [rbp-50h] BYREF
  unsigned int v84; // [rsp+34h] [rbp-4Ch]
  char *v85; // [rsp+38h] [rbp-48h]
  _QWORD *v86; // [rsp+40h] [rbp-40h]
  unsigned __int8 *v87; // [rsp+48h] [rbp-38h]
  int v88; // [rsp+54h] [rbp-2Ch]

  v86 = a3; /*0x103170865*/
  v6 = a1; /*0x103170869*/
  v7 = *((_DWORD *)a2 + 176); /*0x10317086c*/
  *(_OWORD *)a1 = 0; /*0x103170875*/
  *((_QWORD *)a1 + 2) = 0; /*0x103170878*/
  if ( (v7 & 0x41) != 0 )
  {
    v87 = a1; /*0x10317088f*/
    if ( (v7 & 8) != 0 )
    {
      v9 = a2[53]; /*0x103170899*/
      if ( a2[54] != v9 )
      {
        v10 = 12; /*0x1031708ad*/
        v11 = 0; /*0x1031708ba*/
        do
        {
          v12 = *(unsigned int *)(v9 + v10 - 4); /*0x1031708bd*/
          v13 = *(unsigned __int8 *)(v9 + v10 - 8); /*0x1031708c6*/
          v14 = *(_DWORD *)(a2[9] + 4 * v12); /*0x1031708d0*/
          if ( (_DWORD)v12 == *(_DWORD *)(v9 + v10) )
            Luau::formatAppend(
              a1: (_DWORD)v6,
              a2: (unsigned int)"local %d: reg %d, start pc %d line %d, no live range\n",
              a3: v11,
              a4: v13,
              a5: v12,
              a6: v14);
          else
            Luau::formatAppend(
              a1: (_DWORD)v6,
              a2: (unsigned int)"local %d: reg %d, start pc %d line %d, end pc %d line %d\n",
              a3: v11,
              a4: v13,
              a5: v12,
              a6: v14);
          ++v11; /*0x10317090b*/
          v9 = a2[53]; /*0x10317090e*/
          v10 += 16; /*0x103170923*/
        }
        while ( v11 < (a2[54] - v9) >> 4 );
        v7 = *((_DWORD *)a2 + 176); /*0x10317092c*/
      }
    }
    if ( (v7 & 0x20) != 0 )
    {
      v15 = a2[2]; /*0x10317093c*/
      v85 = (char *)(v15 - 23); /*0x103170945*/
      for ( i = 2; ; ++i )
      {
        v17 = *(unsigned __int8 *)(v15 - 24); /*0x10317094c*/
        v18 = (v17 & 1) != 0 ? *(_QWORD *)(v15 - 16) : v17 >> 1;
        if ( v18 <= i ) /*0x103170968*/
          break; /*0x103170968*/
        v19 = v85; /*0x10317096e*/
        if ( (v17 & 1) != 0 ) /*0x103170974*/
          v19 = *(char **)(v15 - 8); /*0x103170976*/
        v20 = v19[i]; /*0x10317097b*/
        v21 = (v20 & 0x7Fu) - 64; /*0x103170986*/
        v22 = a2[65]; /*0x103170989*/
        if ( (a2[66] - v22) >> 5 <= v21 ) /*0x1031709a1*/
          goto LABEL_22; /*0x1031709a1*/
        v23 = v22 + 32 * v21; /*0x1031709a7*/
        if ( (*(_BYTE *)v23 & 1) == 0 ) /*0x1031709af*/
        {
          LODWORD(v23) = v23 + 1; /*0x1031709b1*/
          goto LABEL_23; /*0x1031709b4*/
        }
        v23 = *(_QWORD *)(v23 + 16); /*0x1031709b6*/
        if ( v23 == 0 ) /*0x1031709bd*/
LABEL_22:
          LODWORD(v23) = Luau::getBaseTypeString(this: (Luau *)(unsigned __int8)v20, (unsigned __int8)a2); /*0x1031709c8*/
LABEL_23:
        v24 = "?"; /*0x1031709cb*/
        if ( v20 >= 0 ) /*0x1031709dc*/
          v24 = ""; /*0x1031709dc*/
        Luau::formatAppend(
          a1: (_DWORD)v6,
          a2: (unsigned int)"R%d: %s%s [argument]\n",
          a3: i - 2,
          a4: v23,
          a5: (_DWORD)v24,
          a6);
      }
      v25 = a2[62]; /*0x1031709fd*/
      if ( a2[63] != v25 )
      {
        v26 = 0; /*0x103170a18*/
        while ( 1 )
        {
          v27 = *(_DWORD *)(v25 + 4 * v26); /*0x103170a1b*/
          v28 = (v27 & 0xFFFFFF7F) - 64; /*0x103170a27*/
          v29 = a2[65]; /*0x103170a2a*/
          if ( (a2[66] - v29) >> 5 <= v28 ) /*0x103170a42*/
            break; /*0x103170a42*/
          v30 = v29 + 32 * v28; /*0x103170a48*/
          if ( (*(_BYTE *)v30 & 1) != 0 ) /*0x103170a50*/
          {
            v30 = *(_QWORD *)(v30 + 16); /*0x103170a57*/
            if ( v30 == 0 ) /*0x103170a5e*/
              break; /*0x103170a5e*/
          }
          else
          {
            LODWORD(v30) = v30 + 1; /*0x103170a52*/
          }
LABEL_33:
          v31 = "?"; /*0x103170a6c*/
          if ( (v27 & 0x80u) == 0 ) /*0x103170a7d*/
            v31 = ""; /*0x103170a7d*/
          Luau::formatAppend(a1: (_DWORD)v6, a2: (unsigned int)"U%d: %s%s\n", a3: v26++, a4: v30, a5: (_DWORD)v31, a6);
          v25 = a2[62]; /*0x103170a94*/
          if ( v26 >= (a2[63] - v25) >> 2 ) /*0x103170aac*/
            goto LABEL_36; /*0x103170aac*/
        }
        LODWORD(v30) = Luau::getBaseTypeString(this: (Luau *)(unsigned __int8)v27, (unsigned __int8)a2); /*0x103170a60*/
        goto LABEL_33; /*0x103170a69*/
      }
LABEL_36:
      v32 = a2[59]; /*0x103170ab2*/
      if ( a2[60] != v32 )
      {
        v33 = 12; /*0x103170ac6*/
        v34 = 0; /*0x103170acc*/
        while ( 1 )
        {
          v35 = *(_DWORD *)(v32 + v33 - 12); /*0x103170acf*/
          v36 = (v35 & 0xFFFFFF7F) - 64; /*0x103170adb*/
          v37 = a2[65]; /*0x103170ade*/
          if ( (a2[66] - v37) >> 5 <= v36 ) /*0x103170af6*/
            break; /*0x103170af6*/
          v38 = v37 + 32 * v36; /*0x103170afc*/
          if ( (*(_BYTE *)v38 & 1) != 0 ) /*0x103170b04*/
          {
            v38 = *(_QWORD *)(v38 + 16); /*0x103170b0b*/
            if ( v38 == 0 ) /*0x103170b12*/
              break; /*0x103170b12*/
          }
          else
          {
            LODWORD(v38) = v38 + 1; /*0x103170b06*/
          }
LABEL_43:
          v39 = "?"; /*0x103170b1f*/
          if ( (v35 & 0x80u) == 0 ) /*0x103170b2f*/
            v39 = ""; /*0x103170b2f*/
          v6 = v87; /*0x103170b45*/
          Luau::formatAppend(
            a1: (_DWORD)v87,
            a2: (unsigned int)"R%d: %s%s from %d to %d\n",
            a3: *(unsigned __int8 *)(v32 + v33 - 8),
            a4: v38,
            a5: (_DWORD)v39,
            a6: *(_DWORD *)(v32 + v33 - 4));
          ++v34; /*0x103170b5a*/
          v32 = a2[59]; /*0x103170b5d*/
          v33 += 16; /*0x103170b72*/
          if ( v34 >= (a2[60] - v32) >> 4 ) /*0x103170b79*/
            goto LABEL_46; /*0x103170b79*/
        }
        LODWORD(v38) = Luau::getBaseTypeString(this: (Luau *)(unsigned __int8)v35, (unsigned __int8)a2); /*0x103170b14*/
        goto LABEL_43; /*0x103170b1c*/
      }
    }
LABEL_46:
    v40 = *((_DWORD *)a2 + 176); /*0x103170b7f*/
    if ( (v40 & 0x40) != 0 )
    {
      v41 = a2[13]; /*0x103170b8a*/
      if ( v41 != a2[12] )
      {
        v42 = 0; /*0x103170ba5*/
        do
        {
          Luau::formatAppend(a1: (_DWORD)v6, a2: (unsigned int)"K%d: ", a3: v42, a4: v41, a5, a6);
          (*(void (__fastcall **)(__int64 *, unsigned __int8 *, _QWORD, __int64))(*a2 + 48))( /*0x103170bcd*/
            a1: a2,
            a2: v6,
            a3: (unsigned int)v42,
            a4: 1);
          Luau::formatAppend(a1: (_DWORD)v6, a2: (unsigned int)"\n", a3: v43, a4: v44, a5: v45, a6: v46); /*0x103170bd8*/
          ++v42; /*0x103170be5*/
        }
        while ( v42 < 0xCCCCCCCCCCCCCCCDLL * ((a2[13] - a2[12]) >> 3) );
        v40 = *((_DWORD *)a2 + 176); /*0x103170bf5*/
      }
    }
    if ( (v40 & 1) != 0 )
    {
      v47 = (a2[7] - a2[6]) >> 2; /*0x103170c0c*/
      v83 = -1; /*0x103170c14*/
      std::vector<int>::vector[abi:ne200100](a1: &v81, a2: v47, a3: &v83); /*0x103170c1e*/
      v49 = a2[6]; /*0x103170c23*/
      if ( a2[7] == v49 ) /*0x103170c2b*/
      {
        v52 = 0; /*0x103170c79*/
      }
      else
      {
        v50 = 0; /*0x103170c30*/
        do /*0x103170c75*/
        {
          JumpTarget = Luau::getJumpTarget(this: (Luau *)*(unsigned int *)(v49 + 4 * v50), a2: v50, a3: v48); /*0x103170c3a*/
          if ( JumpTarget >= 0 ) /*0x103170c41*/
            *((_DWORD *)v81 + (unsigned int)JumpTarget) = 0; /*0x103170c49*/
          v50 += (int)Luau::getOpLength(a1: *(unsigned __int8 *)(a2[6] + 4 * v50)); /*0x103170c60*/
          v49 = a2[6]; /*0x103170c63*/
          v52 = (a2[7] - v49) >> 2; /*0x103170c6e*/
        }
        while ( v50 < v52 ); /*0x103170c75*/
      }
      v53 = v81; /*0x103170c7d*/
      if ( v82 != v81 ) /*0x103170c88*/
      {
        v54 = ((v82 - (_BYTE *)v81) >> 2 == 0) + ((v82 - (_BYTE *)v81) >> 2); /*0x103170c92*/
        v55 = 0; /*0x103170c96*/
        v56 = 0; /*0x103170c98*/
        do /*0x103170cab*/
        {
          if ( v53[v55] == 0 ) /*0x103170c9e*/
            v53[v55] = v56++; /*0x103170ca0*/
          ++v55; /*0x103170ca5*/
        }
        while ( v54 != v55 ); /*0x103170cab*/
      }
      v83 = -1; /*0x103170cb4*/
      std::vector<int>::resize(a1: v86, a2: v52 + 1, a3: &v83); /*0x103170cbe*/
      v59 = a2[6]; /*0x103170cc3*/
      v60 = a2[7]; /*0x103170cc7*/
      if ( v60 == v59 )
      {
        v76 = 0; /*0x103170e9c*/
      }
      else
      {
        v85 = (char *)a2 + 649; /*0x103170cdb*/
        v88 = -1; /*0x103170cdf*/
        v61 = 0; /*0x103170ce6*/
        v62 = 0; /*0x103170ce9*/
        do
        {
          v63 = *v6; /*0x103170cec*/
          if ( (v63 & 1) != 0 ) /*0x103170cf2*/
            LODWORD(v64) = *((_DWORD *)v6 + 2); /*0x103170cf4*/
          else
            LODWORD(v64) = v63 >> 1; /*0x103170cf9*/
          v65 = *(unsigned __int8 *)(v59 + 4 * v62); /*0x103170cfb*/
          *(_DWORD *)(*v86 + 4 * v62) = v64; /*0x103170d08*/
          v84 = v65; /*0x103170d0c*/
          if ( v65 == 65 )
          {
            ++v62; /*0x103170d14*/
          }
          else
          {
            v66 = *((_DWORD *)a2 + 176); /*0x103170d1c*/
            if ( (v66 & 0x10) != 0 ) /*0x103170d25*/
            {
              v64 = a2[78]; /*0x103170d27*/
              if ( v61 < (a2[79] - v64) >> 3 ) /*0x103170d3f*/
              {
                do /*0x103170d93*/
                {
                  if ( v62 != *(_DWORD *)(v64 + 8 * v61) ) /*0x103170d48*/
                    break; /*0x103170d48*/
                  LODWORD(v67) = (_DWORD)v85; /*0x103170d52*/
                  if ( (a2[81] & 1) != 0 ) /*0x103170d56*/
                    v67 = a2[83]; /*0x103170d58*/
                  Luau::formatAppend( /*0x103170d73*/
                    a1: (_DWORD)v6,
                    a2: (unsigned int)"REMARK %s\n",
                    a3: v67 + *(_DWORD *)(v64 + 8 * v61++ + 4),
                    a4: v64,
                    a5: v57,
                    a6: v58);
                  v64 = a2[78]; /*0x103170d7b*/
                }
                while ( v61 < (a2[79] - v64) >> 3 ); /*0x103170d93*/
                v66 = *((_DWORD *)a2 + 176); /*0x103170d95*/
              }
            }
            v80 = v59; /*0x103170d9c*/
            if ( (v66 & 4) != 0 )
            {
              v64 = a2[9]; /*0x103170da4*/
              v68 = *(_DWORD *)(v64 + 4 * v62); /*0x103170da8*/
              LOBYTE(v64) = v68 <= 0; /*0x103170daf*/
              if ( v68 > 0 && v68 != v88 )
              {
                v69 = a2[89] + 24LL * (unsigned int)v68; /*0x103170dcb*/
                if ( (*(_BYTE *)(v69 - 24) & 1) != 0 ) /*0x103170dd3*/
                  v69 = *(_QWORD *)(v69 - 8); /*0x103170ddb*/
                else
                  LODWORD(v69) = v69 - 23; /*0x103170dd5*/
                Luau::formatAppend(a1: (_DWORD)v6, a2: (unsigned int)"%5d: %s\n", a3: v68, a4: v69, a5: v57, a6: v58);
                v66 = *((_DWORD *)a2 + 176); /*0x103170df3*/
                v88 = v68; /*0x103170dfa*/
              }
            }
            if ( (v66 & 2) != 0 )
              Luau::formatAppend(
                a1: (_DWORD)v6,
                a2: (unsigned int)"%d: ",
                a3: *(_DWORD *)(a2[9] + 4 * v62),
                a4: v64,
                a5: v57,
                a6: v58);
            v70 = *((_DWORD *)v81 + v62); /*0x103170e1f*/
            if ( v70 != -1 )
              Luau::formatAppend(a1: (_DWORD)v6, a2: (unsigned int)"L%d: ", a3: v70, a4: v64, a5: v57, a6: v58);
            v71 = (unsigned int *)(v80 + 4 * v62); /*0x103170e3d*/
            v72 = Luau::getJumpTarget(this: (Luau *)*v71, a2: v62, a3: v70); /*0x103170e48*/
            if ( v72 < 0 ) /*0x103170e4f*/
              v75 = -1; /*0x103170e5c*/
            else
              v75 = *((_DWORD *)v81 + (unsigned int)v72); /*0x103170e57*/
            Luau::BytecodeBuilder::dumpInstruction(a1: a2, a2: v71, a3: (__int64)v6, a4: v75, a5: v73, a6: v74); /*0x103170e6a*/
            v62 += (int)Luau::getOpLength(a1: v84); /*0x103170e79*/
            v59 = a2[6]; /*0x103170e7c*/
            v60 = a2[7]; /*0x103170e80*/
          }
          v76 = v60 - v59; /*0x103170e87*/
        }
        while ( v62 < (v60 - v59) >> 2 );
      }
      v77 = *v6; /*0x103170e9e*/
      if ( (v77 & 1) != 0 ) /*0x103170ea3*/
        v78 = *((_DWORD *)v6 + 2); /*0x103170ea5*/
      else
        v78 = v77 >> 1; /*0x103170eaa*/
      *(_DWORD *)(*v86 + v76) = v78; /*0x103170eb3*/
      if ( v81 != nullptr ) /*0x103170ebd*/
      {
        v82 = v81; /*0x103170ebf*/
        operator delete(a1: v81); /*0x103170ec3*/
      }
    }
  }
  return v6; /*0x103170ecb*/
}

// Address: 0x1031710b4 | Name: __ZN4Luau15BytecodeBuilder13setDumpSourceERKNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEE
__int64 __fastcall Luau::BytecodeBuilder::setDumpSource(__int64 a1, __int64 a2)
{
  __int64 v4; // rsi
  __int64 v5; // r15
  __int64 v6; // rax
  __int64 v7; // r13
  __int64 v8; // rcx
  __int64 result; // rax
  __int64 v10; // rdx
  _BYTE v11[16]; // [rsp+0h] [rbp-50h] BYREF
  void *v12; // [rsp+10h] [rbp-40h]
  __int64 v13; // [rsp+18h] [rbp-38h]
  _BYTE v14[41]; // [rsp+27h] [rbp-29h] BYREF

  v4 = *(_QWORD *)(a1 + 712); /*0x1031710d2*/
  v13 = a1 + 712; /*0x1031710d9*/
  std::vector<std::string>::__base_destruct_at_end[abi:ne200100](a1: a1 + 712, a2: v4); /*0x1031710dd*/
  v5 = 0; /*0x1031710e2*/
  do /*0x1031711ca*/
  {
    v6 = std::string::find(a1: a2, a2: 10, a3: v5); /*0x1031710f4*/
    if ( v6 == -1 ) /*0x1031710fd*/
    {
      std::string::basic_string(a1: v11, a2, a3: v5, a4: -1, a5: v14); /*0x103171151*/
      std::vector<std::string>::push_back[abi:ne200100](a1: v13, a2: (__int64)v11); /*0x10317115d*/
      if ( (v11[0] & 1) != 0 ) /*0x103171166*/
        operator delete(a1: v12); /*0x10317116c*/
      v5 = -1; /*0x103171171*/
    }
    else
    {
      v7 = v6; /*0x1031710ff*/
      std::string::basic_string(a1: v11, a2, a3: v5, a4: v6 - v5, a5: v14); /*0x103171115*/
      std::vector<std::string>::push_back[abi:ne200100](a1: v13, a2: (__int64)v11); /*0x103171121*/
      if ( (v11[0] & 1) != 0 ) /*0x10317112a*/
        operator delete(a1: v12); /*0x103171130*/
      v5 = v7 + 1; /*0x103171138*/
    }
    v8 = *(_QWORD *)(a1 + 720); /*0x103171178*/
    result = *(unsigned __int8 *)(v8 - 24); /*0x10317117f*/
    if ( (result & 1) != 0 ) /*0x103171185*/
    {
      result = *(_QWORD *)(v8 - 16); /*0x1031711a7*/
      if ( result != 0 ) /*0x1031711ae*/
      {
        v10 = *(_QWORD *)(v8 - 8); /*0x1031711b0*/
        if ( *(_BYTE *)(v10 + result - 1) == 13 ) /*0x1031711b9*/
        {
          *(_QWORD *)(v8 - 16) = --result; /*0x1031711be*/
          goto LABEL_16; /*0x1031711be*/
        }
      }
    }
    else if ( *(_BYTE *)(v8 - 24) != 0 ) /*0x10317118a*/
    {
      result = (unsigned int)result >> 1; /*0x10317118c*/
      if ( *(_BYTE *)(v8 + result - 24) == 13 ) /*0x103171193*/
      {
        *(_BYTE *)(v8 - 24) = 2 * --result; /*0x10317119b*/
        v10 = v8 - 23; /*0x1031711a2*/
LABEL_16:
        *(_BYTE *)(v10 + result) = 0; /*0x1031711c2*/
      }
    }
  }
  while ( v5 != -1 ); /*0x1031711ca*/
  return result; /*0x1031711d0*/
}

// Address: 0x1031711fc | Name: __ZNK4Luau15BytecodeBuilder12dumpFunctionEj
Luau::BytecodeBuilder *__fastcall Luau::BytecodeBuilder::dumpFunction(
        Luau::BytecodeBuilder *this,
        __int64 a2,
        unsigned int a3)
{
  __int64 v4; // rcx
  _OWORD *v5; // rax

  v4 = *(_QWORD *)(a2 + 8); /*0x103171207*/
  v5 = (_OWORD *)(v4 + 136LL * a3 + 40); /*0x103171219*/
  if ( (*(_BYTE *)v5 & 1) != 0 ) /*0x103171220*/
  {
    std::string::__init_copy_ctor_external( /*0x10317123d*/
      a1: this,
      a2: *(_QWORD *)(v4 + 136LL * a3 + 56),
      a3: *(_QWORD *)(v4 + 136LL * a3 + 48));
  }
  else
  {
    *((_QWORD *)this + 2) = *(_QWORD *)(v4 + 136LL * a3 + 56); /*0x103171226*/
    *(_OWORD *)this = *v5; /*0x10317122d*/
  }
  return this; /*0x103171249*/
}

// Address: 0x10317124c | Name: __ZNK4Luau15BytecodeBuilder14dumpEverythingEv
Luau::BytecodeBuilder *__fastcall Luau::BytecodeBuilder::dumpEverything(
        Luau::BytecodeBuilder *this,
        __int64 a2,
        unsigned __int64 a3,
        unsigned __int64 a4,
        int a5,
        int a6)
{
  __int64 v7; // rax
  __int64 v9; // r13
  unsigned __int64 v10; // r12
  __int64 v11; // rax
  char *v12; // rcx
  __int64 v13; // rax
  unsigned int v14; // edx
  __int64 v15; // rsi
  __int64 v16; // rdx
  __int128 v18; // [rsp+0h] [rbp-40h] BYREF
  void *v19; // [rsp+10h] [rbp-30h]

  *(_OWORD *)this = 0; /*0x103171263*/
  *((_QWORD *)this + 2) = 0; /*0x103171266*/
  v7 = *(_QWORD *)(a2 + 8); /*0x10317126e*/
  if ( *(_QWORD *)(a2 + 16) != v7 ) /*0x103171276*/
  {
    v9 = 0; /*0x103171286*/
    v10 = 0; /*0x103171289*/
    do /*0x10317128c*/
    {
      LOBYTE(a4) = *(_BYTE *)(v7 + v9 + 64); /*0x10317128c*/
      if ( (a4 & 1) != 0 ) /*0x103171294*/
      {
        a3 = *(_QWORD *)(v7 + v9 + 72); /*0x1031712b2*/
        if ( a3 == 0 ) /*0x1031712ba*/
        {
LABEL_8:
          std::string::basic_string[abi:ne200100]<0>(a1: &v18, a2: "??", a3, a4); /*0x1031712cc*/
          goto LABEL_9; /*0x1031712d7*/
        }
        std::string::__init_copy_ctor_external(a1: &v18, a2: *(_QWORD *)(v7 + v9 + 80), a3); /*0x1031712c5*/
      }
      else
      {
        if ( (_BYTE)a4 == 0 ) /*0x103171298*/
          goto LABEL_8; /*0x103171298*/
        v11 = v9 + v7 + 64; /*0x10317129d*/
        v19 = *(void **)(v11 + 16); /*0x1031712a5*/
        v18 = *(_OWORD *)v11; /*0x1031712ac*/
      }
LABEL_9:
      v12 = (char *)&v18 + 1; /*0x1031712dc*/
      if ( (v18 & 1) != 0 ) /*0x1031712e4*/
        LODWORD(v12) = (_DWORD)v19; /*0x1031712e6*/
      Luau::formatAppend(a1: (_DWORD)this, a2: (unsigned int)"Function %d (%s):\n", a3: v10, a4: (_DWORD)v12, a5, a6); /*0x1031712f9*/
      v13 = *(_QWORD *)(a2 + 8); /*0x1031712fe*/
      v14 = *(unsigned __int8 *)(v13 + v9 + 40); /*0x103171302*/
      if ( (v14 & 1) != 0 ) /*0x10317130b*/
      {
        v15 = *(_QWORD *)(v13 + v9 + 56); /*0x10317130d*/
        v16 = *(_QWORD *)(v13 + v9 + 48); /*0x103171312*/
      }
      else
      {
        v16 = v14 >> 1; /*0x103171319*/
        v15 = v13 + v9 + 41; /*0x10317131f*/
      }
      std::string::append(a1: this, a2: v15, a3: v16); /*0x103171326*/
      std::string::append(a1: this, a2: "\n"); /*0x103171331*/
      if ( (v18 & 1) != 0 ) /*0x10317133a*/
        operator delete(a1: v19); /*0x103171340*/
      ++v10; /*0x103171345*/
      v7 = *(_QWORD *)(a2 + 8); /*0x103171348*/
      a3 = 0xF0F0F0F0F0F0F0F1LL; /*0x103171357*/
      a4 = 0xF0F0F0F0F0F0F0F1LL * ((*(_QWORD *)(a2 + 16) - v7) >> 3); /*0x103171361*/
      v9 += 136; /*0x103171365*/
    }
    while ( v10 < a4 ); /*0x10317128c*/
  }
  return this; /*0x103171378*/
}

// Address: 0x1031713b4 | Name: __ZNK4Luau15BytecodeBuilder17dumpSourceRemarksEv
Luau::BytecodeBuilder *__fastcall Luau::BytecodeBuilder::dumpSourceRemarks(Luau::BytecodeBuilder *this, _QWORD *a2)
{
  _QWORD *v2; // r15
  unsigned __int64 v3; // rcx
  __int64 v4; // rcx
  int v5; // r9d
  __int64 v6; // rcx
  unsigned __int64 v7; // r14
  unsigned __int64 v8; // r12
  unsigned __int8 *v9; // rdi
  char v10; // al
  __int64 v11; // rdx
  unsigned __int8 *v12; // rsi
  unsigned __int64 i; // r14
  unsigned __int64 v14; // rcx
  unsigned __int8 *v15; // rcx
  unsigned __int8 *v16; // rcx
  __int64 v17; // r8
  __int64 v18; // rbx
  unsigned __int8 v19; // dl
  __int64 v20; // rcx
  __int64 v21; // r8
  unsigned __int64 v22; // r15
  __int64 v23; // rbx
  unsigned __int64 v24; // r13
  unsigned int v25; // eax
  size_t v26; // rdx
  unsigned int v27; // ecx
  __int64 v28; // rsi
  const void *v29; // rdi
  const void *v30; // rsi
  unsigned __int64 v31; // r14
  unsigned __int64 v32; // rax
  __int128 *v34; // [rsp+0h] [rbp-90h] BYREF
  __int64 v35; // [rsp+8h] [rbp-88h]
  unsigned __int64 v36; // [rsp+10h] [rbp-80h]
  _QWORD *v37; // [rsp+18h] [rbp-78h]
  __int128 v38; // [rsp+20h] [rbp-70h] BYREF
  __int64 v39; // [rsp+30h] [rbp-60h]
  unsigned __int8 *v40; // [rsp+38h] [rbp-58h]
  unsigned __int8 *v41; // [rsp+40h] [rbp-50h]
  int v42; // [rsp+4Ch] [rbp-44h]
  unsigned __int64 v43; // [rsp+50h] [rbp-40h]
  Luau::BytecodeBuilder *v44; // [rsp+58h] [rbp-38h]
  char v45[41]; // [rsp+67h] [rbp-29h] BYREF

  v2 = a2; /*0x1031713c5*/
  *((_QWORD *)this + 2) = 0; /*0x1031713ca*/
  v44 = this; /*0x1031713d1*/
  *(_OWORD *)this = 0; /*0x1031713d5*/
  v39 = 0; /*0x1031713dc*/
  v38 = 0; /*0x1031713e0*/
  std::vector<std::pair<int,std::string>>::__init_with_size[abi:ne200100]<std::pair<int,std::string>*,std::pair<int,std::string>*>( /*0x1031713ff*/
    a1: &v38,
    a2: a2[92],
    a3: v2[93],
    a4: (__int64)(v2[93] - a2[92]) >> 5);
  _BitScanReverse64(&v3, (__int64)(*((_QWORD *)&v38 + 1) - v38) >> 5); /*0x103171415*/
  v4 = (2 * ((unsigned int)v3 ^ 0x3F)) ^ 0x7ELL; /*0x10317141e*/
  if ( *((_QWORD *)&v38 + 1) == (_QWORD)v38 ) /*0x103171425*/
    v4 = 0; /*0x103171425*/
  std::__introsort<std::_ClassicAlgPolicy,std::__less<void,void> &,std::pair<int,std::string> *,false>( /*0x103171433*/
    a1: v38,
    a2: *((_QWORD *)&v38 + 1),
    a3: v45,
    a4: v4,
    a5: 1);
  v6 = v2[89]; /*0x103171438*/
  if ( v2[90] != v6 )
  {
    v7 = 0; /*0x103171456*/
    v8 = 0; /*0x103171459*/
    v37 = v2; /*0x10317145c*/
    do
    {
      v43 = v7; /*0x103171460*/
      v9 = (unsigned __int8 *)(v6 + 24 * v7); /*0x103171468*/
      v10 = *v9 & 1; /*0x103171471*/
      v11 = *v9 >> 1; /*0x103171473*/
      v12 = v9 + 1; /*0x103171479*/
      for ( i = 0; ; ++i ) /*0x10317147c*/
      {
        v14 = *v9 >> 1; /*0x10317147f*/
        if ( v10 != 0 ) /*0x103171484*/
          v14 = *((_QWORD *)v9 + 1); /*0x103171486*/
        if ( i >= v14 ) /*0x10317148d*/
          break; /*0x10317148d*/
        v15 = v9 + 1; /*0x10317148f*/
        if ( v10 != 0 ) /*0x103171494*/
          v15 = *((unsigned __int8 **)v9 + 2); /*0x103171496*/
        if ( v15[i] != 32 ) /*0x10317149f*/
        {
          v16 = v9 + 1; /*0x1031714a1*/
          if ( v10 != 0 ) /*0x1031714a6*/
            v16 = *((unsigned __int8 **)v9 + 2); /*0x1031714a8*/
          if ( v16[i] != 9 ) /*0x1031714b1*/
            break; /*0x1031714b1*/
        }
      }
      v17 = v38; /*0x1031714b8*/
      if ( v8 < (__int64)(*((_QWORD *)&v38 + 1) - v38) >> 5 )
      {
        v42 = v43 + 1; /*0x1031714d6*/
        v40 = v9; /*0x1031714d9*/
        v41 = v9 + 1; /*0x1031714dd*/
LABEL_18:
        v18 = 32 * v8; /*0x1031714e1*/
        v19 = *v9; /*0x1031714e8*/
        if ( *(_DWORD *)(v17 + 32 * v8) == v42 )
        {
          LODWORD(v20) = (_DWORD)v12; /*0x1031714f7*/
          if ( (v19 & 1) != 0 ) /*0x1031714fd*/
            v20 = *((_QWORD *)v9 + 2); /*0x1031714ff*/
          v21 = v18 + v17; /*0x103171503*/
          if ( (*(_BYTE *)(v21 + 8) & 1) != 0 ) /*0x10317150b*/
            v21 = *(_QWORD *)(v21 + 24); /*0x103171513*/
          else
            LODWORD(v21) = v21 + 9; /*0x10317150d*/
          Luau::formatAppend(a1: (_DWORD)v44, a2: (unsigned int)"%.*s-- remark: %s\n", a3: i, a4: v20, a5: v21, a6: v5);
          v22 = (__int64)(*((_QWORD *)&v38 + 1) - v38) >> 5; /*0x103171537*/
          v35 = v38; /*0x10317153b*/
          v23 = v38 + v18 + 41; /*0x103171545*/
          v36 = v8; /*0x103171549*/
          while ( 1 )
          {
            v24 = v8++; /*0x10317154d*/
            if ( v8 >= v22 ) /*0x103171556*/
              break; /*0x103171556*/
            if ( *(_DWORD *)(v23 - 9) == *(_DWORD *)(v23 - 41) )
            {
              v25 = *(unsigned __int8 *)(v23 - 1); /*0x103171560*/
              v26 = (v25 & 1) != 0 ? *(_QWORD *)(v23 + 7) : v25 >> 1;
              v27 = *(unsigned __int8 *)(v23 - 33); /*0x103171572*/
              v28 = (v27 & 1) != 0 ? *(_QWORD *)(v23 - 25) : v27 >> 1;
              if ( v26 == v28 )
              {
                v29 = (const void *)v23; /*0x10317158a*/
                if ( (v25 & 1) != 0 ) /*0x10317158f*/
                  v29 = *(const void **)(v23 + 15); /*0x103171591*/
                v30 = (v27 & 1) != 0 ? *(const void **)(v23 - 17) : (const void *)(v23 - 32);
                v23 += 32; /*0x1031715a4*/
                if ( memcmp(__s1: v29, __s2: v30, __n: v26) == 0 ) /*0x1031715af*/
                  continue; /*0x1031715af*/
              }
            }
            v8 = v24 + 1; /*0x1031715b4*/
            v9 = v40; /*0x1031715b7*/
            v12 = v41; /*0x1031715bb*/
            v17 = v35; /*0x1031715bf*/
            goto LABEL_18; /*0x1031715c6*/
          }
          v8 = v36 + 1; /*0x1031715cf*/
          if ( v22 > v36 + 1 ) /*0x1031715d5*/
            v8 = v22; /*0x1031715d5*/
          v9 = v40; /*0x1031715d9*/
          v19 = *v40; /*0x1031715dd*/
          v12 = v41; /*0x1031715df*/
        }
        v10 = v19 & 1; /*0x1031715e5*/
        v11 = v19 >> 1; /*0x1031715e9*/
        v2 = v37; /*0x1031715ec*/
      }
      v31 = v43; /*0x1031715fa*/
      if ( v10 != 0 ) /*0x103171600*/
      {
        v12 = *((unsigned __int8 **)v9 + 2); /*0x103171602*/
        v11 = *((_QWORD *)v9 + 1); /*0x103171606*/
      }
      std::string::append(a1: v44, a2: v12, a3: v11); /*0x10317160e*/
      v7 = v31 + 1; /*0x103171613*/
      v6 = v2[89]; /*0x103171616*/
      v32 = 0xAAAAAAAAAAAAAAABLL * ((v2[90] - v6) >> 3); /*0x10317162b*/
      if ( v7 < v32 ) /*0x103171632*/
      {
        std::string::push_back(a1: v44, a2: 10); /*0x10317163d*/
        v6 = v2[89]; /*0x103171642*/
        v32 = 0xAAAAAAAAAAAAAAABLL * ((v2[90] - v6) >> 3); /*0x103171657*/
      }
    }
    while ( v7 < v32 );
  }
  v34 = &v38; /*0x10317166f*/
  std::vector<std::pair<int,std::string>>::__destroy_vector::operator()[abi:ne200100](a1: &v34); /*0x103171672*/
  return v44; /*0x10317167b*/
}

// Address: 0x1031716c2 | Name: __ZNK4Luau15BytecodeBuilder12dumpTypeInfoEv
Luau::BytecodeBuilder *__fastcall Luau::BytecodeBuilder::dumpTypeInfo(
        Luau::BytecodeBuilder *this,
        __int64 a2,
        __int64 a3,
        __int64 a4,
        int a5,
        int a6)
{
  Luau::BytecodeBuilder *v6; // rbx
  __int64 v7; // rax
  __int64 v8; // rcx
  __int64 v9; // rdi
  __int64 v10; // r15
  int v11; // edx
  int v12; // ecx
  unsigned __int8 v13; // si
  int v14; // r8d
  int v15; // r9d
  __int64 v16; // rax
  __int64 v17; // r13
  __int64 v18; // rbx
  __int64 v19; // rax
  Luau *v20; // rdi
  const char *v21; // r14
  int BaseTypeString; // eax
  int v23; // r8d
  int v24; // r9d
  __int64 v27; // [rsp+8h] [rbp-38h]
  Luau::BytecodeBuilder *v28; // [rsp+10h] [rbp-30h]

  v6 = this; /*0x1031716d3*/
  *(_OWORD *)this = 0; /*0x1031716d9*/
  *((_QWORD *)this + 2) = 0; /*0x1031716dc*/
  v7 = *(_QWORD *)(a2 + 8); /*0x1031716e4*/
  v8 = *(_QWORD *)(a2 + 16); /*0x1031716ec*/
  if ( v8 != v7 )
  {
    v9 = 0; /*0x1031716f9*/
    v28 = v6; /*0x1031716fb*/
    do
    {
      v27 = v9; /*0x103171706*/
      v10 = v7 + 136 * v9 + 112; /*0x103171712*/
      if ( (*(_BYTE *)v10 & 1) != 0 )
      {
        if ( *(_QWORD *)(v7 + 136 * v9 + 120) != 0 )
        {
LABEL_7:
          Luau::formatAppend(a1: (_DWORD)v6, a2: (unsigned int)"%zu: function(", a3: v9, a4: v8, a5, a6);
          if ( (*(_BYTE *)v10 & 1) != 0 ) /*0x10317174b*/
            v16 = *(_QWORD *)(v10 + 16); /*0x103171753*/
          else
            v16 = v10 + 1; /*0x10317174d*/
          v17 = *(unsigned __int8 *)(v16 + 1); /*0x103171757*/
          if ( *(_BYTE *)(v16 + 1) != 0 ) /*0x10317175f*/
          {
            v18 = 0; /*0x103171765*/
            while ( 1 ) /*0x10317176b*/
            {
              v19 = v10 + 1; /*0x10317176b*/
              if ( (*(_BYTE *)v10 & 1) != 0 ) /*0x10317176e*/
                v19 = *(_QWORD *)(v10 + 16); /*0x103171770*/
              v20 = (Luau *)*(unsigned __int8 *)(v19 + v18 + 2); /*0x103171774*/
              v21 = "?"; /*0x10317177c*/
              if ( (char)v20 >= 0 ) /*0x10317178a*/
                v21 = ""; /*0x10317178a*/
              BaseTypeString = Luau::getBaseTypeString(this: v20, a2: v13); /*0x10317178e*/
              Luau::formatAppend( /*0x1031717a6*/
                a1: (_DWORD)v28,
                a2: (unsigned int)"%s%s",
                a3: BaseTypeString,
                a4: (_DWORD)v21,
                a5: v23,
                a6: v24);
              if ( v17 == ++v18 ) /*0x1031717b1*/
                break; /*0x1031717b1*/
              Luau::formatAppend(a1: (_DWORD)v28, a2: (unsigned int)", ", a3: v11, a4: v12, a5: v14, a6: v15); /*0x1031717c0*/
            }
          }
          v6 = v28; /*0x1031717ca*/
          Luau::formatAppend(a1: (_DWORD)v28, a2: (unsigned int)")\n", a3: v11, a4: v12, a5: v14, a6: v15); /*0x1031717da*/
          v7 = *(_QWORD *)(a2 + 8); /*0x1031717e3*/
          v8 = *(_QWORD *)(a2 + 16); /*0x1031717e7*/
        }
      }
      else if ( *(_BYTE *)v10 != 0 ) /*0x103171720*/
      {
        goto LABEL_7; /*0x103171720*/
      }
      v9 = v27 + 1; /*0x1031717eb*/
    }
    while ( v27 + 1 < 0xF0F0F0F0F0F0F0F1LL * ((v8 - v7) >> 3) );
  }
  return v6; /*0x103171816*/
}

// Address: 0x103171846 | Name: __ZN4Luau15BytecodeBuilder14getStringTableEv
Luau::BytecodeBuilder *__fastcall Luau::BytecodeBuilder::getStringTable(Luau::BytecodeBuilder *this, _QWORD *a2)
{
  __int64 v4; // rax
  unsigned __int64 v5; // rcx
  __int64 v6; // rsi
  unsigned __int64 v7; // rdx
  unsigned __int64 v8; // rdx
  __int64 v9; // rdi
  unsigned __int64 v10; // rsi

  *(_OWORD *)this = 0; /*0x103171856*/
  *((_QWORD *)this + 2) = 0; /*0x103171859*/
  std::vector<std::string_view>::resize(a1: this, a2: a2[73]); /*0x103171868*/
  v4 = a2[72]; /*0x10317186d*/
  if ( v4 != 0 ) /*0x103171877*/
  {
    v5 = 0; /*0x103171880*/
    while ( 1 ) /*0x103171889*/
    {
      v6 = *(_QWORD *)(a2[71] + 8 * (v5 >> 6)); /*0x103171889*/
      if ( _bittest64(&v6, v5) ) /*0x103171891*/
        break; /*0x103171891*/
      if ( v4 == ++v5 ) /*0x103171899*/
        return this; /*0x103171899*/
    }
  }
  else
  {
    v5 = 0; /*0x10317189d*/
  }
LABEL_12:
  while ( v5 != v4 ) /*0x103171902*/
  {
    *(_OWORD *)(*(_QWORD *)this + 16LL * (unsigned int)(*(_DWORD *)(a2[68] + 24 * v5 + 16) - 1)) = *(_OWORD *)(a2[68] + 24 * v5); /*0x1031718bd*/
    v7 = v5; /*0x1031718c1*/
    v5 = a2[72]; /*0x1031718c4*/
    v8 = v7 + 1; /*0x1031718cb*/
    if ( v5 <= v8 ) /*0x1031718d1*/
      v5 = v8; /*0x1031718d1*/
    while ( v5 != v8 ) /*0x1031718d8*/
    {
      v9 = *(_QWORD *)(a2[71] + 8 * (v8 >> 6)); /*0x1031718e8*/
      v10 = v8 + 1; /*0x1031718ec*/
      if ( _bittest64(&v9, v8++) ) /*0x1031718f0*/
      {
        v5 = v10 - 1; /*0x1031718fc*/
        goto LABEL_12; /*0x1031718fc*/
      }
    }
  }
  return this; /*0x103171907*/
}

// Address: 0x10317195a | Name: __ZNK4Luau15BytecodeBuilder19annotateInstructionERNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEEjj
void __fastcall Luau::BytecodeBuilder::annotateInstruction(__int64 a1, int a2, unsigned int a3, unsigned int a4)
{
  __int64 v5; // rcx
  __int64 v6; // rdi
  unsigned __int64 v7; // r8
  unsigned int v8; // r9d
  int v9; // edx
  int v10; // eax
  int v11; // edx

  if ( (*(_BYTE *)(a1 + 704) & 1) != 0 ) /*0x103171965*/
  {
    v5 = *(_QWORD *)(a1 + 8) + 136LL * a3; /*0x10317197c*/
    v6 = *(_QWORD *)(v5 + 88); /*0x103171980*/
    v7 = (*(_QWORD *)(v5 + 96) - v6) >> 2; /*0x10317198d*/
    v8 = a4 + 1; /*0x103171991*/
    do /*0x1031719a7*/
    {
      v9 = *(_DWORD *)(v6 + 4LL * v8); /*0x103171998*/
      if ( v7 <= v8 ) /*0x10317199f*/
        break; /*0x10317199f*/
      ++v8; /*0x1031719a1*/
    }
    while ( v9 == -1 ); /*0x1031719a7*/
    v10 = *(_DWORD *)(v6 + 4LL * a4); /*0x1031719ab*/
    v11 = v9 - v10; /*0x1031719af*/
    if ( (*(_BYTE *)(v5 + 40) & 1) != 0 ) /*0x1031719b5*/
      v5 = *(_QWORD *)(v5 + 56); /*0x1031719bd*/
    else
      LODWORD(v5) = v5 + 41; /*0x1031719b7*/
    Luau::formatAppend(a1: a2, a2: (unsigned int)"%.*s", a3: v11, a4: v10 + v5, a5: v7, a6: v8); /*0x1031719d4*/
  }
}

// Address: 0x1031719da | Name: __ZN4Luau15BytecodeBuilderD1Ev
void __fastcall Luau::BytecodeBuilder::~BytecodeBuilder(Luau::BytecodeBuilder *this)
{
  Luau::BytecodeBuilder::~BytecodeBuilder(this); /*0x1031719df*/
}

// Address: 0x1031719e4 | Name: __ZN4Luau15BytecodeBuilderD0Ev
void __fastcall Luau::BytecodeBuilder::~BytecodeBuilder(Luau::BytecodeBuilder *this)
{
  Luau::BytecodeBuilder::~BytecodeBuilder(this); /*0x1031719ed*/
  operator delete(a1: this); /*0x1031719fb*/
}

// Address: 0x10317238e | Name: __ZNK4Luau15BytecodeBuilder11ConstantKeyeqERKS1_
bool __fastcall Luau::BytecodeBuilder::ConstantKey::operator==(__int64 a1, __int64 a2)
{
  return *(_DWORD *)a1 == *(_DWORD *)a2 /*0x1031723c5*/
      && *(_QWORD *)(a1 + 8) == *(_QWORD *)(a2 + 8)
      && *(_QWORD *)(a1 + 16) == *(_QWORD *)(a2 + 16)
      && *(_QWORD *)(a1 + 24) == *(_QWORD *)(a2 + 24)
      && *(_QWORD *)(a1 + 32) == *(_QWORD *)(a2 + 32);
}

// Address: 0x10652fbdc | Name: __ZN4Luau15BytecodeBuilderD2Ev
void __fastcall Luau::BytecodeBuilder::~BytecodeBuilder(void **this)
{
  void *v2; // rdi
  void *v3; // rdi
  void *v4; // rdi
  void *v5; // rdi
  void *v6; // rdi
  void *v7; // rdi
  void *v8; // rdi
  void *v9; // rdi
  void *v10; // rdi
  void *v11; // rdi
  void *v12; // rdi
  void *v13; // rdi
  void *v14; // rdi
  _QWORD v15[3]; // [rsp+8h] [rbp-18h] BYREF

  *this = off_10C8C02F0; /*0x10652fbf5*/
  if ( (*(_BYTE *)(this + 95) & 1) != 0 ) /*0x10652fbff*/
    operator delete(a1: *(this + 97)); /*0x10652fc08*/
  v15[0] = this + 92; /*0x10652fc18*/
  std::vector<std::pair<int,std::string>>::__destroy_vector::operator()[abi:ne200100](a1: v15); /*0x10652fc1e*/
  v15[0] = this + 89; /*0x10652fc2a*/
  std::vector<std::string>::__destroy_vector::operator()[abi:ne200100](a1: v15); /*0x10652fc31*/
  if ( (*(_BYTE *)(this + 85) & 1) != 0 ) /*0x10652fc3d*/
    operator delete(a1: *(this + 87)); /*0x10652fc46*/
  if ( (*(_BYTE *)(this + 81) & 1) != 0 ) /*0x10652fc52*/
    operator delete(a1: *(this + 83)); /*0x10652fc5b*/
  v2 = *(this + 78); /*0x10652fc60*/
  if ( v2 != nullptr ) /*0x10652fc6a*/
  {
    *(this + 79) = v2; /*0x10652fc6c*/
    operator delete(a1: v2); /*0x10652fc73*/
  }
  v3 = *(this + 75); /*0x10652fc78*/
  if ( v3 != nullptr ) /*0x10652fc82*/
  {
    *(this + 76) = v3; /*0x10652fc84*/
    operator delete(a1: v3); /*0x10652fc8b*/
  }
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::StringRef,std::pair<Luau::BytecodeBuilder::StringRef,unsigned int>,std::pair<Luau::BytecodeBuilder::StringRef const,unsigned int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::StringRef,unsigned int>,Luau::BytecodeBuilder::StringRefHash,std::equal_to<Luau::BytecodeBuilder::StringRef>>::~DenseHashTable2(a1: this + 68); /*0x10652fc97*/
  v15[0] = this + 65; /*0x10652fca7*/
  std::vector<Luau::BytecodeBuilder::UserdataType>::__destroy_vector::operator()[abi:ne200100](); /*0x10652fcaa*/
  v4 = *(this + 62); /*0x10652fcaf*/
  if ( v4 != nullptr ) /*0x10652fcb9*/
  {
    *(this + 63) = v4; /*0x10652fcbb*/
    operator delete(a1: v4); /*0x10652fcc2*/
  }
  v5 = *(this + 59); /*0x10652fcc7*/
  if ( v5 != nullptr ) /*0x10652fcd1*/
  {
    *(this + 60) = v5; /*0x10652fcd3*/
    operator delete(a1: v5); /*0x10652fcda*/
  }
  v6 = *(this + 56); /*0x10652fcdf*/
  if ( v6 != nullptr ) /*0x10652fce9*/
  {
    *(this + 57) = v6; /*0x10652fceb*/
    operator delete(a1: v6); /*0x10652fcf2*/
  }
  v7 = *(this + 53); /*0x10652fcf7*/
  if ( v7 != nullptr ) /*0x10652fd01*/
  {
    *(this + 54) = v7; /*0x10652fd03*/
    operator delete(a1: v7); /*0x10652fd0a*/
  }
  Luau::detail::DenseHashTable2<unsigned int,std::pair<unsigned int,short>,std::pair<unsigned int const,short>,Luau::detail::ItemInterfaceMap2<unsigned int,short>,std::hash<unsigned int>,std::equal_to<unsigned int>>::~DenseHashTable2(a1: this + 45); /*0x10652fd16*/
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::TableShape,std::pair<Luau::BytecodeBuilder::TableShape,int>,std::pair<Luau::BytecodeBuilder::TableShape const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::TableShape,int>,Luau::BytecodeBuilder::TableShapeHash,std::equal_to<Luau::BytecodeBuilder::TableShape>>::~DenseHashTable2(a1: this + 38); /*0x10652fd22*/
  Luau::detail::DenseHashTable2<Luau::BytecodeBuilder::ConstantKey,std::pair<Luau::BytecodeBuilder::ConstantKey,int>,std::pair<Luau::BytecodeBuilder::ConstantKey const,int>,Luau::detail::ItemInterfaceMap2<Luau::BytecodeBuilder::ConstantKey,int>,Luau::BytecodeBuilder::ConstantKeyHash,std::equal_to<Luau::BytecodeBuilder::ConstantKey>>::~DenseHashTable2(a1: this + 31); /*0x10652fd2e*/
  v8 = *(this + 27); /*0x10652fd33*/
  if ( v8 != nullptr ) /*0x10652fd3d*/
  {
    *(this + 28) = v8; /*0x10652fd3f*/
    operator delete(a1: v8); /*0x10652fd46*/
  }
  v15[0] = this + 24; /*0x10652fd56*/
  std::vector<Luau::BytecodeBuilder::ClassShape>::__destroy_vector::operator()[abi:ne200100](); /*0x10652fd59*/
  v9 = *(this + 21); /*0x10652fd5e*/
  if ( v9 != nullptr ) /*0x10652fd68*/
  {
    *(this + 22) = v9; /*0x10652fd6a*/
    operator delete(a1: v9); /*0x10652fd71*/
  }
  v10 = *(this + 18); /*0x10652fd76*/
  if ( v10 != nullptr ) /*0x10652fd80*/
  {
    *(this + 19) = v10; /*0x10652fd82*/
    operator delete(a1: v10); /*0x10652fd89*/
  }
  v11 = *(this + 15); /*0x10652fd8e*/
  if ( v11 != nullptr ) /*0x10652fd95*/
  {
    *(this + 16) = v11; /*0x10652fd97*/
    operator delete(a1: v11); /*0x10652fd9e*/
  }
  v12 = *(this + 12); /*0x10652fda3*/
  if ( v12 != nullptr ) /*0x10652fdaa*/
  {
    *(this + 13) = v12; /*0x10652fdac*/
    operator delete(a1: v12); /*0x10652fdb0*/
  }
  v13 = *(this + 9); /*0x10652fdb5*/
  if ( v13 != nullptr ) /*0x10652fdbc*/
  {
    *(this + 10) = v13; /*0x10652fdbe*/
    operator delete(a1: v13); /*0x10652fdc2*/
  }
  v14 = *(this + 6); /*0x10652fdc7*/
  if ( v14 != nullptr ) /*0x10652fdce*/
  {
    *(this + 7) = v14; /*0x10652fdd0*/
    operator delete(a1: v14); /*0x10652fdd4*/
  }
  v15[0] = this + 1; /*0x10652fde1*/
  std::vector<Luau::BytecodeBuilder::Function>::__destroy_vector::operator()[abi:ne200100](); /*0x10652fde4*/
}

// END BytecodeBuilder part 1/1, 85 functions
