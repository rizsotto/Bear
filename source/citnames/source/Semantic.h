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

#include "Configuration.h"
#include "CompilationDatabase.h"
#include "libresult/Result.h"
#include "libreport/Report.h"
#include "libsys/Context.h"

namespace cs {

    struct Semantic {
        virtual ~Semantic() noexcept = default;
        [[nodiscard]] virtual std::list<output::Entry> into_compilation(const cfg::Content&) const = 0;
    };

    using SemanticPtr = std::shared_ptr<Semantic>;

    struct Tool {
        virtual ~Tool() noexcept = default;
        [[nodiscard]] virtual SemanticPtr is_a(const report::Execution::Command&) const = 0;
    };

    using ToolPtr = std::shared_ptr<Tool>;
    using Tools = std::list<ToolPtr>;

    class Expert {
    public:
        static rust::Result<Expert> from(const cfg::Value& cfg);
        static rust::Result<Expert> from(const cfg::Value& cfg, const sys::Context& ctx);

        [[nodiscard]] output::Entries transform(const report::Report& report) const;
        [[nodiscard]] SemanticPtr recognize(const report::Execution::Command&) const;

    public:
        Expert() = delete;
        ~Expert() = default;

        explicit Expert(const cfg::Value&, Tools &&) noexcept;

    private:
        const cfg::Value& config_;
        Tools tools_;
    };
}
