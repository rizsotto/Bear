#pragma once

#include "Tool.h"
#include "Parsers.h"

namespace cs::semantic {

    struct ToolLinker : public Tool {

        [[nodiscard]]
        rust::Result<SemanticPtr> recognize(const Execution &execution) const override;

    protected:
        [[nodiscard]]
        bool is_linker_call(const fs::path& program) const;

        [[nodiscard]]
        rust::Result<SemanticPtr> linking(const Execution &execution) const;

        [[nodiscard]]
        static rust::Result<SemanticPtr> linking(const FlagsByName &flags, const Execution &execution);

        static const FlagsByName FLAG_DEFINITION;
    };
} 