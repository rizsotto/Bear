/*  Copyright (C) 2012-2022 by László Nagy
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

#include "config.h"
#include "Citnames.h"
#include "Output.h"
#include "semantic/Build.h"
#include "semantic/Tool.h"
#include "collect/db/EventsDatabaseReader.h"
#include "libconfig/Configuration.h"

#include <filesystem>

#ifdef HAVE_FMT_STD_H
#include <fmt/std.h>
#endif
#include <fmt/ostream.h>
#include <spdlog/spdlog.h>

namespace db = ic::collect::db;

namespace {

size_t transform(cs::semantic::Build &build, const db::EventsDatabaseReader::Ptr& events, std::list<cs::Entry> &output) {
        for (const auto &event : *events) {
            const auto entries = build.recognize(event)
                    .map<std::list<cs::Entry>>([](const auto &semantic) -> std::list<cs::Entry> {
                        const auto candidate = dynamic_cast<const cs::semantic::CompilerCall *>(semantic.get());
                        return (candidate != nullptr) ? candidate->into_entries() : std::list<cs::Entry>();
                    })
                    .unwrap_or({});
            std::copy(entries.begin(), entries.end(), std::back_inserter(output));
        }
        return output.size();
    }
}

namespace cs {

    rust::Result<int> Command::execute() const {
        cs::CompilationDatabase output(configuration_.output.format, configuration_.output.content);
        std::list<cs::Entry> entries;

        // get current compilations from the input.
        return db::EventsDatabaseReader::from(configuration_.input_file)
                .map<size_t>([this, &entries](const auto &commands) {
                    cs::semantic::Build build(configuration_.compilation);
                    return transform(build, commands, entries);
                })
                .and_then<size_t>([this, &output, &entries](auto new_entries_count) {
                    std::error_code error_code;
                    spdlog::debug("compilation entries created. [size: {}]", new_entries_count);
                    // read back the current content and extend with the new elements.
                    return (configuration_.append && fs::exists(configuration_.output_file, error_code))
                        ? output.from_json(configuration_.output_file.c_str(), entries)
                                .template map<size_t>([&new_entries_count](auto old_entries_count) {
                                    spdlog::debug("compilation entries have read. [size: {}]", old_entries_count);
                                    return new_entries_count + old_entries_count;
                                })
                        : rust::Result<size_t>(rust::Ok(new_entries_count));
                })
                .and_then<size_t>([this, &output, &entries](const size_t & size) {
                    // write the entries into the output file.
                    spdlog::debug("compilation entries to output. [size: {}]", size);
                    return output.to_json(configuration_.output_file.c_str(), entries);
                })
                .map<int>([](auto size) {
                    // just map to success exit code if it was successful.
                    spdlog::debug("compilation entries written. [size: {}]", size);
                    return EXIT_SUCCESS;
                });
    }

    Command::Command(config::Citnames configuration) noexcept
            : ps::Command()
            , configuration_(std::move(configuration))
    { }

    Citnames::Citnames(const config::Citnames &config, const ps::ApplicationLogConfig& log_config) noexcept
            : ps::SubcommandFromConfig<config::Citnames>("citnames", log_config, config)
    { }

    rust::Result<ps::CommandPtr> Citnames::command(const config::Citnames &config) const {
        return rust::Ok<ps::CommandPtr>(std::make_unique<Command>(config));
    }

    std::optional<std::runtime_error> Citnames::update_config(const flags::Arguments &args) {
        return config_.update(args);
    }
}
