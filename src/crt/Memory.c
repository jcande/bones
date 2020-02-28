#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>

#include "Memory.h"

#define BITSIZE(t)      (8 * sizeof(t))
#define AS_INDEX(head) ((head) / BITSIZE(Cell))
#define AS_OFFSET(head) ((head) % BITSIZE(Cell))

int
MemInit(
    Memory *State,
    Cell *RawMemory,
    uint64_t RawMemorySize
    )
{
    int status = 0;

    if (State == NULL)
    {
        status = ENV_BADPTR;
        TagWarnx("NULL MemInit pointer");
        goto Bail;
    }

    memset(State, 0, sizeof(*State));
    State->memory = RawMemory;
    State->memorySize = RawMemorySize;
    // Place head in the middle of memory
    State->head = (State->memorySize * BITSIZE(Cell)) / 2;
    // Ensure the cache head is different. This signals that the cache is
    // invalid.
    State->cache.head = !State->head;

    status = 0;

Bail:
    return status;
}

void
MemSync(
    Memory *State
    )
{
    if (AS_INDEX(State->head) != AS_INDEX(State->cache.head))
    {
        assert(AS_INDEX(State->head) >= 0 && "Ensure head hasn't fallen off the left side of the tape.");
        assert(AS_INDEX(State->head) < State->memorySize && "Ensure head hasn't fallen off the right side of the tape.");

        if (State->cache.dirty)
        {
            //printf("flushing %lu (index: %lu): %lx\n", State->cache.head, AS_INDEX(State->cache.head), State->cache.value);
            State->memory[AS_INDEX(State->cache.head)] = State->cache.value;
            State->cache.dirty = false;
        }

        State->cache.head = State->head;
        State->cache.value = State->memory[AS_INDEX(State->head)];
        //printf("Cached %lu (index: %lu): %lx\n", State->cache.head, AS_INDEX(State->cache.head), State->cache.value);
    }
    //else { printf("HIT %lu\n", State->head); }
}

void
MemWrite(
    Memory *State,
    bool Bit
    )
{
    uint64_t bit = (Bit != false);
    MemSync(State);

    uint8_t offset = AS_OFFSET(State->head);
    State->cache.value &= ~(1UL << offset);
    State->cache.value |= bit << offset;
    State->cache.dirty = true;
    //printf("WRITE %lu (index: %lu): %lx\n", State->cache.head, AS_INDEX(State->cache.head), State->cache.value);
}

bool
MemRead(
    Memory *State
    )
{
    MemSync(State);

    uint8_t offset = AS_OFFSET(State->head);
    return (State->cache.value & (1UL << offset)) != 0;
}
