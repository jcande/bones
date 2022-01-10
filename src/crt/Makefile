CC := clang
#CFLAGS += -g -ggdb -O0 -std=c11 -W -Wall -Wextra -pedantic
CFLAGS += -Os -std=c11 -W -Wall -Wextra -pedantic

TARGET := wm

SRC := $(wildcard *.c)
OBJ := $(SRC:%.c=.obj/%.o)
DEP := $(SRC:%.c=.obj/%.d)

.PHONY: clean all default

all: $(TARGET)
clean:
	$(RM) $(TARGET)
	$(RM) -r .obj

# Dependencies tracking
$(foreach BIN,$(TARGET),$(eval $(BIN): $(OBJ)))

$(TARGET): $(OBJ)
	$(LINK.o) -o $@ $^

$(OBJ) $(DEP): | .obj
.obj:
	echo $(SRC)
	mkdir $@
.obj/%.o: %.c
	$(COMPILE.c) -MMD -MP -o $@ $<

-include $(DEP)
