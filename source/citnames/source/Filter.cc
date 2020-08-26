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

#include "Filter.h"

namespace {

    struct NoFilter : public cs::output::Filter {
        bool operator()(const cs::output::Entry &) noexcept override {
            return true;
        }
    };

    struct StrictFilter : public cs::output::Filter {

        explicit StrictFilter(cs::output::Content config)
                : config_(std::move(config))
        { }

        bool operator()(const cs::output::Entry &entry) noexcept override {
            const bool exists = is_exists(entry.file);

            const auto &include = config_.paths_to_include;
            const bool to_include = include.empty() || contains(include, entry.file);
            const auto &exclude = config_.paths_to_exclude;
            const bool to_exclude = !exclude.empty() && contains(exclude, entry.file);

            return exists && to_include && !to_exclude;
        }

        static bool is_exists(const fs::path& path)
        {
            std::error_code error_code;
            return fs::exists(path, error_code);
        }

        static bool contains(const fs::path& root, const fs::path& file)
        {
            auto [root_end, nothing] = std::mismatch(root.begin(), root.end(), file.begin());
            return (root_end == root.end());
        }

        static bool contains(const std::list<fs::path>& root, const fs::path& file)
        {
            return root.end() != std::find_if(root.begin(), root.end(),
                                              [&file](auto directory) { return contains(directory, file); });
        }

    private:
        cs::output::Content config_;
    };
}


namespace cs::output {
    FilterPtr make_filter(const cs::output::Content &cfg)
    {
        return (cfg.include_only_existing_source)
               ? FilterPtr(new StrictFilter(cfg))
               : FilterPtr(new NoFilter());
    }
}
