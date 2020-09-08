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

#include "Tool.h"
#include "ToolGcc.h"
#include "ToolCuda.h"

#include <filesystem>
#include <functional>
#include <unordered_map>
#include <unordered_set>
#include <stdexcept>
#include <utility>

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

namespace fs = std::filesystem;

namespace {

    template<typename Entry, typename Id>
    struct Forest {
        Forest(std::list<Entry> const &input,
               std::function<Id(Entry const &)> id_extractor,
               std::function<Id(Entry const &)> parent_id_extractor);

        template<typename Output>
        std::list<Output> bfs(std::function<rust::Result<std::list<Output>>(const Entry &)>) const;

    private:
        std::unordered_map<Id, Entry const*> entries;
        std::unordered_map<Id, std::list<Id>> nodes;
        std::list<Id> roots;
    };

    template<typename Entry, typename Id>
    Forest<Entry, Id>::Forest(std::list<Entry> const &input,
                              std::function<Id(Entry const &)> id_extractor,
                              std::function<Id(Entry const &)> parent_id_extractor)
            : entries()
            , nodes()
            , roots()
    {
        std::unordered_set<Id> maybe_roots;
        std::unordered_set<Id> non_roots;
        for (Entry const &entry : input) {
            Id const id = id_extractor(entry);
            Id const parent = parent_id_extractor(entry);
            // emplace into the entry map
            entries.emplace(std::make_pair(id, &entry));
            // put into the nodes map, if it's not yet exists
            if (auto search = nodes.find(id); search == nodes.end()) {
                std::list<Id> children = {};
                nodes.emplace(std::make_pair(id, children));
            }
            // update (or create) the parent element with the new child
            if (auto search = nodes.find(parent); search != nodes.end()) {
                search->second.push_back(id);
            } else {
                std::list<Id> children = {id};
                nodes.emplace(std::make_pair(parent, children));
            }
            // update the root nodes
            if (maybe_roots.count(id) != 0) {
                maybe_roots.erase(id);
            }
            non_roots.insert(id);
            if (non_roots.count(parent) == 0) {
                maybe_roots.insert(parent);
            }
        }
        // fixing the phantom root node which has no entry
        std::unordered_set<Id> new_roots;
        for (auto root : maybe_roots) {
            if (auto phantom = entries.find(root); phantom == entries.end()) {
                auto children = nodes.at(root);
                std::copy(children.begin(), children.end(), std::inserter(new_roots, new_roots.begin()));
                nodes.erase(root);
            } else {
                new_roots.insert(root);
            }
        }
        // set the root nodes as an ordered list
        std::copy(new_roots.begin(), new_roots.end(), std::back_inserter(roots));
        roots.sort();
    }

    template<typename Entry, typename Id>
    template<typename Output>
    std::list<Output> Forest<Entry, Id>::bfs(std::function<rust::Result<std::list<Output>>(const Entry &)> function) const {
        std::list<Output> result;
        // define a work queue
        std::list<Id> queue = roots;
        while (!queue.empty()) {
            // get the pivot id
            Id id = queue.front();
            queue.pop_front();
            // get the entry for the id
            auto entry = entries.at(id);
            function(*entry)
                    .on_success([&result](const auto& outputs) {
                        // if we found the semantic for an entry, we add that to the output.
                        // and we don't process the children processes.
                        std::copy(outputs.begin(), outputs.end(), std::back_inserter(result));
                    })
                    .on_error([this, &queue, &id](const auto&) {
                        // if it did not recognize the entry, we continue to process the
                        // child processes.
                        const auto ids = nodes.at(id);
                        std::copy(ids.begin(), ids.end(), std::back_inserter(queue));
                    });
        }
        return result;
    }
}

namespace cs::semantic {

    Tools::Tools(ToolPtrs &&tools, std::list<fs::path> compilers) noexcept
            : tools_(tools), compilers_(std::move(compilers)) {}

    rust::Result<Tools> Tools::from(const cfg::Compilation &cfg) {
        ToolPtrs tools = {
                std::make_shared<ToolGcc>(),
                std::make_shared<ToolCuda>(),
        };
        return rust::Ok(Tools(std::move(tools), cfg.compilers));
    }

    output::Entries Tools::transform(const report::Report &report) const {
        auto semantics =
                Forest<report::Execution, report::Pid>(
                        report.executions,
                        [](report::Execution const &execution) -> report::Pid { return execution.run.pid; },
                        [](report::Execution const &execution) -> report::Pid { return execution.run.ppid; }
                ).bfs<SemanticPtr>([this](const auto &command) {
                    return this->recognize(command);
                });

        output::Entries result;
        for (const auto &semantic : semantics) {
            if (auto candidate = semantic->into_entry(); candidate) {
                result.emplace_back(candidate.value());
            }
        }
        return result;
    }

    [[nodiscard]]
    rust::Result<SemanticPtrs> Tools::recognize(const report::Execution &execution) const {
        spdlog::debug("[pid: {}] command: {}", execution.run.pid, execution.command);
        return select(execution.command)
                .on_success([&execution](auto tool) {
                    spdlog::debug("[pid: {}] recognized with: {}", execution.run.pid, tool->name());
                })
                .and_then<SemanticPtrs>([&execution](auto tool) {
                    return tool->compilations(execution.command);
                })
                .on_success([&execution](auto items) {
                     spdlog::debug("[pid: {}] recognized as: [{}]", execution.run.pid, items);
                })
                .on_error([&execution](const auto &error) {
                    spdlog::debug("[pid: {}] failed: {}", execution.run.pid, error.what());
                });
    }

    [[nodiscard]]
    rust::Result<Tools::ToolPtr> Tools::select(const report::Command &command) const {
        // do different things if the command is matching one of the nominated compilers.
        if (compilers_.end() != std::find(compilers_.begin(), compilers_.end(), command.program)) {
            // TODO: if interaction with the system is allowed, then it would be more
            //       precise if it cann call the command with `--version` to get which
            //       tool shall it call, instead of call all of them.

            // check if any tools can get the semantic
            for (const auto &tool : tools_) {
                if (auto candidate = tool->compilations(command); candidate.is_ok()) {
                    return rust::Ok(tool);
                }
            }
        } else {
            // check if any tool can recognize the command.
            for (const auto &tool : tools_) {
                // when the tool is matching...
                if (tool->recognize(command.program)) {
                    return rust::Ok(tool);
                }
            }
        }
        return rust::Err(std::runtime_error("No tools recognize this command."));
    }
}
