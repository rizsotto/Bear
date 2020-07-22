/*  Copyright (C) 2012-2020 by László Nagy
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

#include "Semantic.h"

namespace cs {

    rust::Result<Semantic> Semantic::from(const cfg::Configuration& cfg)
    {
        // TODO
        return rust::Ok(Semantic());
    }

    rust::Result<Semantic> Semantic::from(const cfg::Configuration& cfg, const sys::Context& ctx)
    {
        // TODO
        return rust::Ok(Semantic());
    }

    output::Entries Semantic::run(const report::Report& report) const
    {
        // TODO:
        //  - filter for successful executions
        //    - shall reconstruct process call chain
        //  - filter for recognized compilers
        //  - split command and arguments
        //  - parse arguments
        //  - create an entry
        //  - filter out non existing source files
        return output::Entries();
    }
}