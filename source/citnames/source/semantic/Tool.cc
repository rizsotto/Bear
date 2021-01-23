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
#include "ToolClang.h"
#include "ToolCuda.h"
#include "ToolWrapper.h"
#include "ToolExtendingWrapper.h"
#include "Convert.h"

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

    // Represent a process tree.
    //
    // Processes have parent process (which started). If all process execution
    // could have been captured this is a single process tree. But because some
    // execution might escape (static executables are not visible for dynamic
    // loader) the tree falls apart into a forest.
    //
    // Why create the process forest?
    //
    // The idea helps to filter out such executions which are not relevant to the
    // user. If a compiler executes itself (with different set of arguments) it
    // will cause duplicate entries, which is not desirable. (CUDA compiler is
    // is a good example to call GCC multiple times.)
    //
    // First we build up the forest, then starting on a single process tree, we
    // do a breadth first search. If a process can be identified (recognized as
    // compilation) we don't inspect the children processes.
    template<typename Entry, typename Id>
    struct Forest {

        template<
                typename Iterator,
                typename Extractor = std::function<rust::Result<std::tuple<Entry, Id, Id>>(typename Iterator::reference)>
                >
        Forest(Iterator begin, Iterator end, Extractor extractor);

        template<
                typename Output,
                typename Converter = std::function<rust::Result<std::list<Output>>(const Entry &, Id)>
                >
        std::list<Output> bfs(Converter) const;

    private:
        std::unordered_map<Id, Entry> entries;
        std::unordered_map<Id, std::list<Id>> nodes;
        std::list<Id> roots;
    };

    template<typename Entry, typename Id>
    template<typename Iterator, typename Extractor>
    Forest<Entry, Id>::Forest(Iterator begin, Iterator end, Extractor extractor) {
        std::unordered_set<Id> maybe_roots;
        std::unordered_set<Id> non_roots;
        for (auto it = begin; it != end; ++it) {
            extractor(*it)
                .on_success([this, &maybe_roots, &non_roots](auto tuple) {
                    const auto &[entry, id, parent] = tuple;
                    // emplace into the entry map
                    entries.emplace(std::make_pair(id, entry));
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
                })
                .on_error([](auto error) {
                    spdlog::warn("Could not read value from database: {}", error.what());
                });
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
    template<typename Output, typename Converter>
    std::list<Output> Forest<Entry, Id>::bfs(Converter function) const {
        std::list<Output> result;
        // define a work queue
        std::list<Id> queue = roots;
        while (!queue.empty()) {
            // get the pivot id
            Id id = queue.front();
            queue.pop_front();
            // get the entry for the id
            auto entry = entries.at(id);
            function(entry, id)
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

    rust::Result<domain::Run> extract(cs::EventsIterator::reference input) {
        using Result = rust::Result<domain::Run>;
        return input
                .and_then<domain::Run>([](auto events) {
                    if (events.empty()) {
                        return Result(
                                rust::Err(
                                        std::runtime_error("Event list is empty.")
                                )
                        );
                    }
                    if (auto start = events.front(); !start->has_started()) {
                        return Result(
                                rust::Err(
                                        std::runtime_error("Could not find start event.")
                                )
                        );
                    } else {
                        const auto &started = start->started();
                        return Result(
                                rust::Ok(
                                        domain::Run{
                                                domain::from(started.execution()),
                                                started.pid(),
                                                started.ppid()
                                        }
                                )
                        );
                    }
                });
    }
}

namespace cs::semantic {

    Tools::Tools(ToolPtrs &&tools, std::list<fs::path>&& compilers) noexcept
            : tools_(tools)
            , to_exclude_(compilers)
    {}

    rust::Result<Tools> Tools::from(Compilation cfg) {
        // TODO: use `cfg.flags_to_remove`
        ToolPtrs tools = {
                std::make_shared<ToolGcc>(),
                std::make_shared<ToolClang>(),
                std::make_shared<ToolWrapper>(),
                std::make_shared<ToolCuda>(),
        };
        for (auto && compiler : cfg.compilers_to_recognize) {
            tools.emplace_back(std::make_shared<ToolExtendingWrapper>(std::move(compiler)));
        }

        return rust::Ok(Tools(std::move(tools), std::move(cfg.compilers_to_exclude)));
    }

    Entries Tools::transform(cs::EventsDatabase::Ptr events) const {
        auto semantics =
                Forest<Execution, uint32_t>(
                        events->events_by_process_begin(),
                        events->events_by_process_end(),
                        ::extract
                ).bfs<SemanticPtr>([this](const auto &execution, const auto pid) {
                    return this->recognize(execution, pid);
                });

        Entries result;
        for (const auto &semantic : semantics) {
            if (auto candidate = semantic->into_entry(); candidate) {
                result.emplace_back(candidate.value());
            }
        }
        return result;
    }

    [[nodiscard]]
    rust::Result<SemanticPtrs> Tools::recognize(const Execution &execution, const uint32_t pid) const {
        spdlog::debug("[pid: {}] execution: {}", pid, execution);
        return select(execution)
                .on_success([&pid](auto tool) {
                    spdlog::debug("[pid: {}] recognized with: {}", pid, tool->name());
                })
                .and_then<SemanticPtrs>([&execution](auto tool) {
                    return tool->compilations(execution);
                })
                .on_success([&pid](auto items) {
                     spdlog::debug("[pid: {}] recognized as: [{}]", pid, items);
                })
                .on_error([&pid](const auto &error) {
                    spdlog::debug("[pid: {}] failed: {}", pid, error.what());
                });
    }

    [[nodiscard]]
    rust::Result<Tools::ToolPtr> Tools::select(const Execution &execution) const {
        // do different things if the execution is matching one of the nominated compilers.
        if (to_exclude_.end() != std::find(to_exclude_.begin(), to_exclude_.end(), execution.executable)) {
            return rust::Err(std::runtime_error("The compiler is on the exclude list from configuration."));
        } else {
            // check if any tool can recognize the execution.
            for (const auto &tool : tools_) {
                // when the tool is matching...
                if (tool->recognize(execution.executable)) {
                    return rust::Ok(tool);
                }
            }
        }
        return rust::Err(std::runtime_error("No tools recognize this execution."));
    }
}
