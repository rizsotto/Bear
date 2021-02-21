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

#pragma once

#include <queue>
#include <mutex>
#include <optional>
#include <condition_variable>

namespace domain {

    template <class T>
    class ThreadSafeQueue {
    public:
        ThreadSafeQueue() = default;
        ~ThreadSafeQueue() noexcept = default;

        void push(T &&value) noexcept {
            std::lock_guard<std::mutex> lock(mutex);
            queue.emplace(value);
            cv.notify_one();
        }

        void flush() noexcept {
            std::lock_guard<std::mutex> lock(mutex);
            wake = true;
            cv.notify_one();
        }

        [[nodiscard]]
        std::optional<T> pop() noexcept {
            std::unique_lock<std::mutex> lock(mutex);
            cv.wait(lock, [this]() { return wake || (!queue.empty()); });

            if (queue.empty()) {
                return std::nullopt;
            } else {
                auto value = std::make_optional(queue.front());
                queue.pop();
                return value;
            }
        }

    private:
        std::mutex mutex;
        std::condition_variable cv;
        std::queue<T> queue;
        bool wake = false;
    };
}
