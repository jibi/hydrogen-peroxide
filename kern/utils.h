// Copyright (C) 2020 Gilberto "jibi" Bertin <me@jibi.io>
//
// This file is part of hydrogen peroxyde.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long u64;
typedef int i32;

/* A single val map is just a convenient macro used to define single value array
 * maps for storing mostly configuration values.
 */
#define SINGLE_VAL_MAP(_name, _value_size)	\
struct bpf_map_def SEC("maps") _name = {	\
	.type = BPF_MAP_TYPE_ARRAY,		\
	.key_size = sizeof(u32),		\
	.value_size = sizeof(_value_size),	\
	.max_entries = 1,			\
};

#define get_val(type, key) ({					\
	void *val = bpf_map_lookup_elem(&key, &(u32){0});	\
	if (!val)						\
		return XDP_PASS;				\
								\
	*(type *)val;						\
})
