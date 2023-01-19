#include "libconfig/Citnames-config.h"

#include <libsys/Path.h>
#include <libsys/Environment.h>
#include <fmt/format.h>
#include <nlohmann/json.hpp>
#include <spdlog/spdlog.h>

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
#include <fmt/ostream.h>
template <> struct fmt::formatter<config::Citnames> : ostream_formatter {};
#endif

namespace config {

    void from_json(const nlohmann::json &j, Format &rhs) {
        j.at("command_as_array").get_to(rhs.command_as_array);
        j.at("drop_output_field").get_to(rhs.drop_output_field);
    }

    void to_json(nlohmann::json &j, const Format &rhs) {
        j = nlohmann::json{
                {"command_as_array",  rhs.command_as_array},
                {"drop_output_field", rhs.drop_output_field},
        };
    }

    void from_json(const nlohmann::json &j, Content &rhs) {
        j.at("include_only_existing_source").get_to(rhs.include_only_existing_source);

        if (j.contains("paths_to_include")) {
            j.at("paths_to_include").get_to(rhs.paths_to_include);
        }
        if (j.contains("paths_to_exclude")) {
            j.at("paths_to_exclude").get_to(rhs.paths_to_exclude);
        }
        if (j.contains("duplicate_filter_fields")) {
            j.at("duplicate_filter_fields").get_to(rhs.duplicate_filter_fields);
        }
    }

    void to_json(nlohmann::json &j, const Content &rhs) {
        j = nlohmann::json{
                {"include_only_existing_source", rhs.include_only_existing_source},
                {"duplicate_filter_fields", rhs.duplicate_filter_fields},
        };
        if (!rhs.paths_to_include.empty()) {
            j["paths_to_include"] = rhs.paths_to_include;
        }
        if (!rhs.paths_to_exclude.empty()) {
            j["paths_to_exclude"] = rhs.paths_to_exclude;
        }
    }

    void from_json(const nlohmann::json &j, Output &rhs) {
        j.at("format").get_to(rhs.format);
        j.at("content").get_to(rhs.content);
    }

    void to_json(nlohmann::json &j, const Output &rhs) {
        j = nlohmann::json{
                {"format",  rhs.format},
                {"content", rhs.content},
        };
    }

    void from_json(const nlohmann::json &j, CompilerWrapper &rhs) {
        j.at("executable").get_to(rhs.executable);

        if (j.contains("flags_to_add")) {
            j.at("flags_to_add").get_to(rhs.flags_to_add);
        }
        if (j.contains("flags_to_remove")) {
            j.at("flags_to_remove").get_to(rhs.flags_to_remove);
        }
    }

    void to_json(nlohmann::json &j, const CompilerWrapper &rhs) {
        j = nlohmann::json{
                {"executable",  rhs.executable},
        };
        if (!rhs.flags_to_add.empty()) {
            j["flags_to_add"] = rhs.flags_to_add;
        }
        if (!rhs.flags_to_remove.empty()) {
            j["flags_to_remove"] = rhs.flags_to_remove;
        }
    }

    void from_json(const nlohmann::json &j, Compilation &rhs) {
        if (j.contains("compilers_to_recognize")) {
            j.at("compilers_to_recognize").get_to(rhs.compilers_to_recognize);
        }
        if (j.contains("compilers_to_exclude")) {
            j.at("compilers_to_exclude").get_to(rhs.compilers_to_exclude);
        }
    }

    void to_json(nlohmann::json &j, const Compilation &rhs) {
        if (!rhs.compilers_to_recognize.empty()) {
            j["compilers_to_recognize"] = rhs.compilers_to_recognize;
        }
        if (!rhs.compilers_to_exclude.empty()) {
            j["compilers_to_exclude"] = rhs.compilers_to_exclude;
        }
    }

    void from_json(const nlohmann::json &j, Citnames &rhs) {
        if (j.contains("output")) {
            j.at("output").get_to(rhs.output);
        }
        if (j.contains("compilation")) {
            j.at("compilation").get_to(rhs.compilation);
        }
        if (j.contains("input_file")) {
            j.at("input_file").get_to(rhs.input_file);
        }
        if (j.contains("output_file")) {
            j.at("output_file").get_to(rhs.output_file);
        }
        if (j.contains("append")) {
            j.at("append").get_to(rhs.append);
        }
    }

    void to_json(nlohmann::json &j, const Citnames &rhs) {
        j = nlohmann::json{
                {"output",  rhs.output},
                {"compilation", rhs.compilation},
                {"input_file", rhs.input_file},
                {"output_file", rhs.output_file},
                {"append", rhs.append}
        };
    }

    std::optional<std::runtime_error> Citnames::update(const flags::Arguments& args) {
        auto input_arg = args.as_string(cmd::citnames::FLAG_INPUT);
        auto output_arg = args.as_string(cmd::citnames::FLAG_OUTPUT);
        auto append_arg = args.as_bool(cmd::citnames::FLAG_APPEND);
        auto run_checks_arg = args.as_bool(cmd::citnames::FLAG_RUN_CHECKS);

        if (output_arg.is_ok()) {
            output_file = output_arg.unwrap();
        }

        if (input_arg.is_ok()) {
            input_file = input_arg.unwrap();
        } else {
            return std::runtime_error("Missing input file");
        }

        if (append_arg.is_ok()) {
            append = append_arg.unwrap();
        }

        if (run_checks_arg.is_ok()) {
            output.content.include_only_existing_source = run_checks_arg.unwrap();
        }

        if (output.content.include_only_existing_source) {
            output.content.paths_to_exclude = sys::path::to_abspath(output.content.paths_to_exclude)
                .unwrap_or_else([this](const auto& error){
                    spdlog::warn("Conversion to absolute path failed: {}",error.what());
                    return output.content.paths_to_exclude;
                });

            output.content.paths_to_include = sys::path::to_abspath(output.content.paths_to_include)
                .unwrap_or_else([this](const auto& error){
                    spdlog::warn("Conversion to absolute path failed: {}", error.what());
                    return output.content.paths_to_include;
                });
        }

        auto enviroment = sys::env::get();
        std::list<std::string_view> compiler_enviroment_vars = { "CC", "CXX", "FC" };
        for (const auto &var : enviroment) {

            const bool recognized_compiler =
                    std::any_of(compiler_enviroment_vars.begin(), compiler_enviroment_vars.end(),
                            [var](const auto& rhs) { return var.first == rhs; });
            if (!recognized_compiler) {
                continue;
            }

            const bool already_in_wrappers =
                    std::any_of(compilation.compilers_to_recognize.begin(), compilation.compilers_to_recognize.end(),
                                [var](auto wrapper) { return wrapper.executable == var.second; });
            if (already_in_wrappers) {
                continue;
            }

            compilation.compilers_to_recognize.emplace_back(CompilerWrapper {
                var.second,
                {},
                {}
            });

            compiler_enviroment_vars.remove(var.first);
            if (compiler_enviroment_vars.empty()) {
                break;
            }
        }

        spdlog::debug("Parsed configuration: {}", *this);
        return std::nullopt;
    }

    std::ostream &operator<<(std::ostream &os, const Format &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Content &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Output &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const CompilerWrapper &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Compilation &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }

    std::ostream &operator<<(std::ostream &os, const Citnames &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }
}
