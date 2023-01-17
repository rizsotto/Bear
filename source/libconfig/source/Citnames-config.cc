#include "libconfig/citnames-config.h"

#include <fmt/format.h>
#include <nlohmann/json.hpp>

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
    }

    void to_json(nlohmann::json &j, const Citnames &rhs) {
        j = nlohmann::json{
                {"output",  rhs.output},
                {"compilation", rhs.compilation},
        };
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
