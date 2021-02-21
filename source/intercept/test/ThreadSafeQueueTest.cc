/*  Copyright (C) 2012-2021 by László Nagy
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

#include "gtest/gtest.h"

#include "ThreadSafeQueue.h"
#include "ThreadSafeQueueConsumer.h"

namespace {

    TEST(queue, push_and_pop_works) {
        domain::ThreadSafeQueue<int> sut;

        sut.push(1);
        sut.push(2);

        EXPECT_EQ(std::make_optional(1), sut.pop());
        EXPECT_EQ(std::make_optional(2), sut.pop());
    }

    TEST(queue, flush_unblocks) {
        domain::ThreadSafeQueue<int> sut;
        sut.flush();
        EXPECT_EQ(std::nullopt, sut.pop());
        EXPECT_EQ(std::nullopt, sut.pop());
    }

    TEST(queue, flush_unblocks_but_returns_value) {
        domain::ThreadSafeQueue<int> sut;

        sut.push(1);
        sut.flush();
        sut.push(2);

        EXPECT_EQ(std::make_optional(1), sut.pop());
        EXPECT_EQ(std::make_optional(2), sut.pop());
        EXPECT_EQ(std::nullopt, sut.pop());
        EXPECT_EQ(std::nullopt, sut.pop());
    }

    TEST(queue, consumed_from_another_thread) {
        std::vector<int> results;
        {
            domain::ThreadSafeQueueConsumer<int> sut(
                    [&results](auto entry) {
                        std::this_thread::sleep_for(std::chrono::milliseconds (100));
                        results.emplace_back(entry);
                    });

            sut.push(1);
            sut.push(2);
            sut.push(4);
        }
        std::vector<int> expected = { 1, 2, 4 };
        EXPECT_EQ(expected, results);
    }
}
