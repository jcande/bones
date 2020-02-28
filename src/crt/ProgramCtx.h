#pragma once

#include "Environment.h"

// ops

#define SHIFTL(amount) (Env->memory.head -= amount)
#define SHIFTR(amount) (Env->memory.head += amount)

#define SET()       MemWrite(&Env->memory, true)
#define UNSET()     MemWrite(&Env->memory, false)

#define JMP(t, f)   {                               \
    if (MemRead(&Env->memory)) {                    \
        goto t;                                     \
    } else {                                        \
        goto f;                                     \
    }                                               \
}

#define INPUT()     {                               \
    bool bit;                                       \
    CHECK(IoBufferGetBit(&Env->io, &bit));          \
    MemWrite(&Env->memory, bit);                    \
}
#define OUTPUT()    {                               \
    bool bit;                                       \
    bit = MemRead(&Env->memory);                    \
    CHECK(IoBufferPutBit(&Env->io, bit));           \
}


void
Program(
    Environment *Env
    );
