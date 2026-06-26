// This file is part of UltraVNC
// https://github.com/ultravnc/UltraVNC
// https://uvnc.com/
//
// SPDX-License-Identifier: GPL-3.0-or-later
//
// SPDX-FileCopyrightText: Copyright (C) 2002-2025 UltraVNC Team Members. All Rights Reserved.
// SPDX-FileCopyrightText: Copyright (C) 1999-2002 Vdacc-VNC & eSVNC Projects. All Rights Reserved.
//


// CRYPTO LIBRARY FOR EXCHANGING KEYS
// USING THE DIFFIE-HELLMAN KEY EXCHANGE PROTOCOL

// The diffie-hellman can be used to securely exchange keys
// between parties, where a third party eavesdropper given
// the values being transmitted cannot determine the key.

// Implemented by Lee Griffiths, Jan 2004.
// This software is freeware, you may use it to your discretion,
// however by doing so you take full responsibility for any damage
// it may cause.

// Hope you find it useful, even if you just use some of the functions
// out of it like the prime number generator and the XtoYmodN function.

// It would be great if you could send me emails to: lee.griffiths@first4internet.co.uk
// with any suggestions, comments, or questions!

// Enjoy.

// Adopted to MS-Logon for UltraVNC by marscha, 2006.

#ifndef __RFB_DH_H__
#define __RFB_DH_H__

#include <cstdint>
#include <cwchar>
#include <cstdlib>
#include <cstdio>
#include <cstring>
#include <ctime>

using DWORD = std::uint32_t;
using WCHAR = wchar_t;

#define DH_MAX_BITS 31
#define DH_RANGE 100

#define DH_CLEAN_ALL_MEMORY				1
#define DH_CLEAN_ALL_MEMORY_EXCEPT_KEY		2

#define DH_MOD	1
#define DH_GEN	2
#define DH_PRIV	3
#define DH_PUB	4
#define DH_KEY	5

class DH
{
public:
	DH();
	DH(std::uint64_t generator, std::uint64_t modulus);
	~DH();

	void createKeys();
	std::uint64_t createInterKey();
	std::uint64_t createEncryptionKey(std::uint64_t interKey);
	
	std::uint64_t getValue(DWORD flags = DH_KEY);

private:
	std::uint64_t XpowYmodN(std::uint64_t x, std::uint64_t y, std::uint64_t N);
	std::uint64_t generatePrime();
	std::uint64_t tryToGeneratePrime(std::uint64_t start);
	bool millerRabin (std::uint64_t n, unsigned int trials);
	void cleanMem(DWORD flags=DH_CLEAN_ALL_MEMORY);


	std::uint64_t gen;
	std::uint64_t mod;
	std::uint64_t priv;
	std::uint64_t pub;
	std::uint64_t key;
	std::uint64_t maxNum;

};

int bits(std::int64_t number);
bool int64ToBytes(const std::uint64_t integer, char* const bytes);
std::uint64_t bytesToInt64(const char* const bytes);
bool vncWc2Mb(char* multibyte, WCHAR* widechar, int length, int widechar_count);

#endif // __RFB_DH_H__