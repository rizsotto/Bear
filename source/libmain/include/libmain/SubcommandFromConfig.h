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

#include "config.h"
#include "libmain/Application.h"
#include "libmain/ApplicationLogConfig.h"
#include "libresult/Result.h"
#include "libflags/Flags.h"
#include <string>

namespace ps {

    template<typename ConfigType>
    struct SubcommandFromConfig : Subcommand {
        explicit SubcommandFromConfig(const char* name, const ApplicationLogConfig &log_config, ConfigType config) noexcept
            : Subcommand()
            , name_(name)
            , log_config_(log_config)
            , config_(config)
        { }

        bool matches(const flags::Arguments &args) {
            return args.as_string(flags::COMMAND).unwrap_or("") == name_;
        }

        void load_config(const ConfigType &config) {
            config_ = config;
        }

        rust::Result<CommandPtr> subcommand(const flags::Arguments &args) override {
            if (args.as_bool(flags::VERBOSE).unwrap_or(false)) {
                log_config_.initForVerbose();
            } else {
                log_config_.initForSilent();
            }

            if (auto error = update_config(args); error) {
                return rust::Err(*error);
            }

            return this->command(config_);
        }


        virtual rust::Result<CommandPtr> command(const ConfigType &config) const = 0;

        NON_DEFAULT_CONSTRUCTABLE(SubcommandFromConfig)

    protected:
		std::string name_;
        ApplicationLogConfig log_config_;
        ConfigType config_;

    private:
        virtual std::optional<std::runtime_error> update_config(const flags::Arguments &args) = 0;
    };
}
