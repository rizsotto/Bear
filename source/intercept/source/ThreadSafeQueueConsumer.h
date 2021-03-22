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

#include "ThreadSafeQueue.h"

#include <thread>
#include <atomic>
#include <functional>
#include <memory>

namespace domain {

    template <class T>
    class ThreadSafeQueueConsumer {
    public:
        explicit ThreadSafeQueueConsumer(std::function<void(const T &)> consume)
                : queue()
                , running(true)
                , consumer([this, consume]() { loop(consume); })
        { }

        virtual ~ThreadSafeQueueConsumer() noexcept {
            running = false;
            queue.flush();
            consumer.join();
        };

        void push(const T &value) noexcept {
            std::unique_ptr<T> copy = std::make_unique<T>(value);
            queue.push(std::move(copy));
        }

    private:
        void loop(std::function<void(const T&)> consume) noexcept {
            auto value = queue.pop();
            while (running || value) {
                if (value) {
                    consume(*value);
                }
                value = queue.pop();
            }
        }

    private:
        ThreadSafeQueue<T> queue;
        std::atomic<bool> running;
        std::thread consumer;
    };
}
