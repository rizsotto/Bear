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

#include "Reporter.h"

namespace ic {

    Execution::Builder::Builder()
            : execution_(nullptr)
    {
    }

    Execution::Builder& Execution::Builder::add(supervise::Event const& event)
    {
        //    // Create an instance if it's not yet there.
        //    if (execution_.get() == nullptr) {
        //        execution_.reset(new Execution());
        //    }
        //    ::Execution &execution = *execution_;
        // TODO
        return *this;
    }

    ExecutionPtr Execution::Builder::build() noexcept
    {
        return execution_;
    }

    void Reporter::report(const ExecutionPtr& execution) noexcept
    {
    }
}
