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

#include <libconfig.h>

typedef struct bear_output_filter_t bear_output_filter_t;


bear_output_filter_t * bear_filter_read_from_file(char const * file);
bear_output_filter_t * bear_filter_create(config_t const * config);

void bear_filter_delete(bear_output_filter_t * filter);
void bear_filter_report(bear_output_filter_t * filter);

char const * bear_filter_source_file(bear_output_filter_t * filter, bear_message_t const * e);
