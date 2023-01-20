#include "libconfig/Intercept-config.h"

#include <fmt/format.h>
#include <nlohmann/json.hpp>
#include <spdlog/spdlog.h>

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
#include <fmt/ostream.h>
template <> struct fmt::formatter<config::Intercept> : ostream_formatter {};
#endif

namespace config {
	
    void from_json(const nlohmann::json &j, Intercept &rhs) {
        if (j.contains("output_file")) {
            j.at("output_file").get_to(rhs.output_file);
        }
        if (j.contains("library")) {
            j.at("library").get_to(rhs.library);
        }
        if (j.contains("wrapper")) {
            j.at("wrapper").get_to(rhs.wrapper);
        }
        if (j.contains("wrapper_dir")) {
            j.at("wrapper_dir").get_to(rhs.wrapper_dir);
        }
		if (j.contains("command")) {
			j.at("command").get_to(rhs.command);
		}
        if (j.contains("use_preload")) {
            j.at("use_preload").get_to(rhs.use_preload);
        }
        if (j.contains("use_wrapper")) {
            j.at("use_wrapper").get_to(rhs.use_wrapper);
        }
        if (j.contains("verbose")) {
            j.at("verbose").get_to(rhs.verbose);
        }
    }

    void to_json(nlohmann::json &j, const Intercept &rhs) {
        j = nlohmann::json{
                {"output_file",  rhs.output_file},
                {"command", rhs.command},
                {"use_preload", rhs.use_preload},
                {"use_wrapper", rhs.use_wrapper},
                {"verbose", rhs.verbose},
        };
    }

	std::optional<std::runtime_error> Intercept::update(const flags::Arguments& args) {
		args.as_string(cmd::intercept::FLAG_OUTPUT).unwrap_to(output_file);
		args.as_string(cmd::intercept::FLAG_LIBRARY).unwrap_to(library);
		args.as_string(cmd::intercept::FLAG_WRAPPER).unwrap_to(wrapper);
		args.as_string(cmd::intercept::FLAG_WRAPPER_DIR).unwrap_to(wrapper_dir);
        args.as_bool(flags::VERBOSE).unwrap_to(verbose);

		auto force_preload = args.as_bool(cmd::intercept::FLAG_FORCE_PRELOAD);
		auto force_wrapper = args.as_bool(cmd::intercept::FLAG_FORCE_WRAPPER);
		auto command_arg = args.as_string_list(cmd::intercept::FLAG_COMMAND);

		if (force_preload.is_ok() && force_preload.unwrap()) {
			use_wrapper = false;
		}
		if (force_wrapper.is_ok() && force_wrapper.unwrap()) {
			use_preload = false;
		}
		if (!use_preload && !use_wrapper) {
			return std::runtime_error("At least one interception method must be enabled");
		}

		if (command_arg.is_ok()) {
			command.clear();
			for (const auto& cmd_part : command_arg.unwrap()) {
				command.emplace_back(cmd_part);
			}
		}
		if (command.empty()) {
			return std::runtime_error("Missing command to be intercepted");
		}

		return std::nullopt;
	}

    std::ostream &operator<<(std::ostream &os, const Intercept &value)
    {
        nlohmann::json payload;
        to_json(payload, value);
        os << payload;

        return os;
    }
}
