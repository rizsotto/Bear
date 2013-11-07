/*  Copyright (C) 2012, 2013 by László Nagy
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

#include "filter.h"
#include "stringarray.h"

#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/types.h>
#include <regex.h>


typedef struct regex_list_t
{
    size_t    length;
    regex_t * regexs;
    size_t    total_count;
    size_t    match_count;
} regex_list_t;

static void compile(config_setting_t const * array, regex_list_t * prepared);
static int  match(regex_list_t * prepared, char const * input);
static int  is_empty(regex_list_t const * prepared);
static void release(regex_list_t * prepared);

static config_setting_t const * get_setting(config_setting_t const *, char const *);


struct bear_output_filter_t
{
    regex_list_t compilers;
    regex_list_t source_files;
    regex_list_t cancel_parameters;
};


static char const * fix_path(char const * file, char const * cwd);

bear_output_filter_t * bear_filter_read_from_file(char const * file)
{
    config_t config;
    config_init(&config);
    if (config_read_file(&config, file) == CONFIG_FALSE)
    {
        fprintf(stderr, "bear: failed to configure: '%s' in file %s at line %d\n",
                config_error_text(&config),
                config_error_file(&config),
                config_error_line(&config));
        exit(EXIT_FAILURE);
    }
    bear_output_filter_t * const result = bear_filter_create(&config);
    config_destroy(&config);

    return result;
}

bear_output_filter_t * bear_filter_create(config_t const * config)
{
    bear_output_filter_t * filter = malloc(sizeof(bear_output_filter_t));
    if (0 == filter)
    {
        perror("bear: malloc");
        exit(EXIT_FAILURE);
    }

    config_setting_t * const group = config_lookup(config, "filter");
    if (0 == group)
    {
        fprintf(stderr, "bear: found no filter group in config file.\n");
        exit(EXIT_FAILURE);
    }

    compile(get_setting(group, "compilers"), &filter->compilers);
    compile(get_setting(group, "source_files"), &filter->source_files);
    compile(get_setting(group, "cancel_parameters"), &filter->cancel_parameters);

    if (is_empty(&filter->compilers) || is_empty(&filter->source_files))
    {
        fprintf(stderr, "bear: empty compilers or source files in config file will produce empty output.\n");
        exit(EXIT_FAILURE);
    }

    return filter;
}

void bear_filter_report(bear_output_filter_t * filter)
{
    if (0 == filter)
        fprintf(stderr, "bear: filtering were not enabled.\n");
    else
    {
        fprintf(stderr, "bear: filtering statistic:\n");
        fprintf(stderr, "  total number of child processes : %zu\n", filter->compilers.total_count);
        fprintf(stderr, "  match as compiler               : %zu\n", filter->compilers.match_count);
        fprintf(stderr, "  match as source file            : %zu\n", filter->source_files.match_count);
        fprintf(stderr, "  match on cancel parameter       : %zu\n", filter->cancel_parameters.match_count);
    }
}

void bear_filter_delete(bear_output_filter_t * filter)
{
    if (0 == filter)
        return;

    release(&filter->compilers);
    release(&filter->source_files);
    release(&filter->cancel_parameters);

    free((void *)filter);
}

char const * bear_filter_source_file(bear_output_filter_t * filter, bear_message_t const * e)
{
    char const * result = 0;
    // looking for compiler name
    if ((e->cmd) && (e->cmd[0]) && match(&filter->compilers, e->cmd[0]))
    {
        // looking for source file
        char const * const * it = e->cmd;
        for (; *it; ++it)
        {
            if ((0 == result) && match(&filter->source_files, *it))
            {
                result = fix_path(*it, e->cwd);
            }
            else if (match(&filter->cancel_parameters, *it))
            {
                if (result)
                {
                    free((void *)result);
                    result = 0;
                }
                break;
            }
        }
    }
    return result;
}


static void compile(config_setting_t const * array, regex_list_t * prepared)
{
    prepared->total_count = 0;
    prepared->match_count = 0;
    prepared->length = config_setting_length(array);
    if (0 == prepared->length)
    {
        prepared->regexs = 0;
        return;
    }

    prepared->regexs = malloc(prepared->length * sizeof(regex_t));

    size_t idx = 0;
    regex_t * ot = prepared->regexs;
    for (; idx < prepared->length; ++idx, ++ot)
    {
        char const * const it = config_setting_get_string_elem(array, idx);
        int const result = regcomp(ot, it, REG_EXTENDED);
        if (0 != result)
        {
            size_t const errbuf_size = 256;
            char errbuf[errbuf_size];
            regerror(result, ot, errbuf, errbuf_size);
            fprintf(stderr, "bear: regcomp failed on '%s': %s\n", it, errbuf);
            exit(EXIT_FAILURE);
        }
    }
}

static int match(regex_list_t * prepared, char const * input)
{
    ++prepared->total_count;
    size_t idx = 0;
    for (; idx < prepared->length; ++idx)
    {
        regex_t * ot = prepared->regexs + idx;
        if (0 == regexec(ot, input, 0, 0, 0))
        {
            ++prepared->match_count;
            return 1;
        }
    }
    return 0;
}

static int is_empty(regex_list_t const * prepared)
{
    return (prepared->length == 0);
}

static void release(regex_list_t * prepared)
{
    size_t idx = 0;
    for (; idx < prepared->length; ++idx)
    {
        regex_t * ot = prepared->regexs + idx;
        regfree(ot);
    }
    free((void *)prepared->regexs);
}

static config_setting_t const * get_setting(config_setting_t const * config, char const * name)
{
    config_setting_t const * const result = config_setting_get_member(config, name);
    if (0 == result)
    {
        fprintf(stderr, "bear: could not find values for '%s' in file %s.\n",
                name,
                config_setting_source_file(config));
        exit(EXIT_FAILURE);
    }
    if (! config_setting_is_array(result))
    {
        fprintf(stderr, "bear: value for '%s' shall be array of strings in file %s at line %d.\n",
                name,
                config_setting_source_file(result),
                config_setting_source_line(result));
        exit(EXIT_FAILURE);
    }

    size_t const size = config_setting_length(result);
    size_t idx = 0;
    for (; idx < size; ++idx)
    {
        config_setting_t * const elem = config_setting_get_elem(result, idx);
        if (CONFIG_TYPE_STRING != config_setting_type(elem))
        {
            fprintf(stderr, "bear: value for '%s' shall be array of strings in file %s at line %d.\n",
                    name,
                    config_setting_source_file(elem),
                    config_setting_source_line(elem));
            exit(EXIT_FAILURE);
        }
    }
    return result;
}

static char const * fix_path(char const * file, char const * cwd)
{
    char * result = 0;
    if ('/' == file[0])
    {
        result = strdup(file);
        if (0 == result)
        {
            perror("bear: strdup");
            exit(EXIT_FAILURE);
        }
    }
    else
    {
        if (-1 == asprintf(&result, "%s/%s", cwd, file))
        {
            perror("bear: asprintf");
            exit(EXIT_FAILURE);
        }
    }
    return result;
}
