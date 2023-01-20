/*  Copyright (C) 2023 by Samu698
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

#include "libmain/SubcommandFromConfig.h"
#include "libresult/Result.h"
#include "libconfig/Configuration.h"

#include <optional>

namespace cs {

    struct Citnames : ps::SubcommandFromConfig<config::Citnames> {
        Citnames(const config::Citnames &config, const ps::ApplicationLogConfig&) noexcept;

        rust::Result<ps::CommandPtr> command(const config::Citnames &config) const override;

        NON_DEFAULT_CONSTRUCTABLE(Citnames)

    private:
        std::optional<std::runtime_error> update_config(const flags::Arguments &args) override;
    };
}
