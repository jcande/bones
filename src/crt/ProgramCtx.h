#pragma once

#include "Environment.h"

// ops

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wgnu-statement-expression"

#define SHIFTL(amount) ({ Env->memory.head -= amount; })
#define SHIFTR(amount) ({ Env->memory.head += amount; })

#define SET()       ({ MemWrite(&Env->memory, true); })
#define UNSET()     ({ MemWrite(&Env->memory, false); })

#define JMP(t, f)   ({                              \
    if (MemRead(&Env->memory)) {                    \
        goto t;                                     \
    } else {                                        \
        goto f;                                     \
    }                                               \
})

#define INPUT()     ({                              \
    bool bit;                                       \
    CHECK(IoBufferGetBit(&Env->io, &bit));          \
    MemWrite(&Env->memory, bit);                    \
})
#define OUTPUT()    ({                              \
    bool bit;                                       \
    bit = MemRead(&Env->memory);                    \
    CHECK(IoBufferPutBit(&Env->io, bit));           \
})

#pragma clang diagnostic pop


void
Program(
    Environment *Env
    );
