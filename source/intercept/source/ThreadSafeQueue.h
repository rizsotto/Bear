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
#include <memory>
#include <condition_variable>

namespace domain {

    template <class T>
    class ThreadSafeQueue {
    public:
        ThreadSafeQueue() = default;
        ~ThreadSafeQueue() noexcept = default;

        void push(std::unique_ptr<T> value) noexcept {
            std::lock_guard<std::mutex> lock(mutex);
            queue.push(std::move(value));
            cv.notify_one();
        }

        void flush() noexcept {
            std::lock_guard<std::mutex> lock(mutex);
            wake = true;
            cv.notify_one();
        }

        [[nodiscard]]
        std::unique_ptr<T> pop() noexcept {
            std::unique_lock<std::mutex> lock(mutex);
            cv.wait(lock, [this]() { return wake || (!queue.empty()); });

            if (queue.empty()) {
                return std::unique_ptr<T>();
            } else {
                std::unique_ptr<T> value(queue.front().release());
                queue.pop();
                return value;
            }
        }

    private:
        std::mutex mutex;
        std::condition_variable cv;
        std::queue<std::unique_ptr<T>> queue;
        bool wake = false;
    };
}
