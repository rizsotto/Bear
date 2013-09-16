/*  Copyright (C) 2012, 2013 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#pragma once

#include "protocol.h"

typedef struct bear_output_config_t
{
    char const ** compilers;
    char const ** extensions;
} bear_output_config_t;

typedef struct bear_output_t bear_output_t;


bear_output_t * bear_open_json_output(char const * file, bear_output_config_t const *);

void bear_append_json_output(bear_output_t * handle, bear_message_t const * e);
void bear_close_json_output(bear_output_t * handle);
