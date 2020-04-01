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

#pragma once

#include "Flags.h"
#include "Result.h"
#include "Session.h"

#include "supervise.pb.h"

#include <memory>
#include <string>

namespace ic {

    struct Execution {
        class Builder;

        // TODO define types and attributes
        using SharedPtr = std::shared_ptr<Execution>;
    };

    class Execution::Builder {
    public:
        Builder();
        ~Builder() = default;

        Builder& add(supervise::Event const& event);

        Execution::SharedPtr build();

    private:
        Execution::SharedPtr execution_;
    };

    // Will be responsible to append execution to the output
    struct Reporter {
        void report(const Execution::SharedPtr& execution);

        using SharedPtr = std::shared_ptr<Reporter>;
        static rust::Result<SharedPtr> from(const flags::Arguments&);
    };
}
