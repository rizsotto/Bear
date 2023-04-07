/*  Copyright (C) 2012-2023 by László Nagy
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

#include "config.h"
#include "Configuration.h"
#include "semantic/Tool.h"
#include "intercept.grpc.pb.h"

#include <memory>

namespace cs::semantic {

    // Represents an expert system which can recognize compilation entries from
    // command executions. It covers multiple tools and consider omit results
    // based on configuration.
    class Build {
    public:
        explicit Build(Compilation cfg) noexcept;

        [[nodiscard]]
        rust::Result<SemanticPtr> recognize(const rpc::Event &event, const BuildTarget target) const;

        NON_DEFAULT_CONSTRUCTABLE(Build)
        NON_COPYABLE_NOR_MOVABLE(Build)

    private:
        std::shared_ptr<Tool> tools_;
    };
}
