/****************************************************************************
 *
 *   Copyright (c) 2023 PX4 Development Team. All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in
 *    the documentation and/or other materials provided with the
 *    distribution.
 * 3. Neither the name PX4 nor the names of its contributors may be
 *    used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 *
 ****************************************************************************/

#pragma once

#include "ParamLayer.h"

#include <string.h>

#include <px4_platform_common/atomic.h>
#include <px4_platform_common/log.h>

class DynamicSparseLayer : public ParamLayer
{
public:
	// Heap'li kurucu. malloc kurucuda değil, ilk store()'da: statik kurucular
	// SRAM4 eklenmeden önce çalışıyor.
	DynamicSparseLayer(ParamLayer *parent, int n_prealloc = 32, int n_grow = 4) : ParamLayer(parent),
		_n_slots(0), _n_grow(n_grow > 0 ? n_grow : 1), _n_prealloc(n_prealloc > 0 ? n_prealloc : 1),
		_owned(true)
	{
	}

	// Çağıranın verdiği tampon. free() edilmez; boot varsayılanları için.
	DynamicSparseLayer(ParamLayer *parent, void *storage, int n_slots) : ParamLayer(parent),
		_n_slots(n_slots), _n_grow(0), _n_prealloc(n_slots), _owned(false)
	{
		static_assert(sizeof(Slot) == 8, "Slot boyutu runtime_default_mem ile aynı olmalı");
		Slot *slots = static_cast<Slot *>(storage);

		for (int i = 0; i < n_slots; i++) {
			slots[i] = {UINT16_MAX, param_value_u{}};
		}

		_slots.store(slots);
	}

	virtual ~DynamicSparseLayer()
	{
		if (!_owned) {
			return;
		}

		Slot *slots = _slots.load();
		_slots.store(nullptr);
		_next_slot = 0;
		_n_slots = 0;
		free(slots);
	}

	bool store(param_t param, param_value_u value) override
	{
		AtomicTransaction transaction;

		// _grow() malloc için kilidi bırakır; o pencerede başka bir yazıcı
		// aynı parametreyi eklemiş veya yeri doldurmuş olabilir. Büyütme
		// sonrası kararı sıfırdan veririz (PX4 02ecfd4).
		while (true) {
			if (_slots.load() == nullptr && !_alloc_initial(transaction)) {
				return false;
			}

			Slot *slots = _slots.load();
			const int index = _getIndex(param);

			if (index < _next_slot) {
				slots[index].value = value;
				return true;
			}

			if (_next_slot < _n_slots) {
				slots[_next_slot++] = {param, value};
				_sort();
				return true;
			}

			if (!_grow(transaction)) {
				return false;
			}
		}
	}

	bool contains(param_t param) const override
	{
		const AtomicTransaction transaction;
		return _getIndex(param) < _next_slot;
	}

	px4::AtomicBitset<PARAM_COUNT> containedAsBitset() const override
	{
		px4::AtomicBitset<PARAM_COUNT> set;
		const AtomicTransaction transaction;
		Slot *slots = _slots.load();

		for (int i = 0; i < _next_slot; i++) {
			set.set(slots[i].param);
		}

		return set;
	}

	param_value_u get(param_t param) const override
	{
		const AtomicTransaction transaction;
		Slot *slots = _slots.load();

		const int index = _getIndex(param);

		if (index < _next_slot) { // exists in our data structure
			return slots[index].value;
		}

		return _parent->get(param);
	}

	void reset(param_t param) override
	{
		const AtomicTransaction transaction;
		int index = _getIndex(param);
		Slot *slots = _slots.load();

		if (index < _next_slot) {
			slots[index] = {UINT16_MAX, param_value_u{}};
			_sort();
			_next_slot--;
		}
	}

	void refresh(param_t param) override
	{
		_parent->refresh(param);
	}

	int size() const override
	{
		return _next_slot;
	}

	int byteSize() const override
	{
		return _n_slots * sizeof(Slot);
	}

private:
	struct Slot {
		param_t param;
		param_value_u value;
	};

	static int _slotCompare(const void *a, const void *b)
	{
		return ((int)((Slot *)a)->param) - ((int)((Slot *)b)->param);
	}

	void _sort()
	{
		qsort(_slots.load(), _n_slots, sizeof(Slot), _slotCompare);
	}

	int _getIndex(param_t param) const
	{
		int left = 0;
		int right = _next_slot - 1;
		Slot *slots = _slots.load();

		while (left <= right) {
			int mid = (left + right) / 2;

			if (slots[mid].param == param) {
				return mid;

			} else if (slots[mid].param < param) {
				left = mid + 1;

			} else {
				right = mid - 1;
			}
		}

		return _next_slot;
	}

	bool _alloc_initial(AtomicTransaction &transaction)
	{
		if (_slots.load() != nullptr) {
			return true;
		}

		const int n = _n_prealloc;
		transaction.unlock();
		Slot *slots = (Slot *)malloc(sizeof(Slot) * n);
		transaction.lock();

		if (slots == nullptr) {
			PX4_ERR("param layer initial alloc %d failed", n);
			return false;
		}

		// Kilidi bırakırken başka bir yazıcı aynı tamponu kurmuş olabilir.
		if (_slots.load() != nullptr) {
			free(slots);
			return true;
		}

		for (int i = 0; i < n; i++) {
			slots[i] = {UINT16_MAX, param_value_u{}};
		}

		_slots.store(slots);
		_n_slots = n;
		return true;
	}

	bool _grow(AtomicTransaction &transaction)
	{
		if (_n_slots == 0 || _n_grow == 0) {
			return false;
		}

		// PX4 02ecfd4: işaretçi CAS yerine kapasite karşılaştırması.
		// Eski döngü `compare_exchange(&_slots, new)` kullanıyordu; malloc
		// az önce serbest bırakılan adresi geri verince CAS "değişmedi"
		// sanıp yürürlükteki tamponu free() ediyordu (ABA). Ölçüm:
		// runtime_defaults 0x38008240 hâlâ _slots'ta ama heap tahsisli=0,
		// aynı boyutta malloc aynı adresi döndürdü.
		while (_next_slot >= _n_slots) {
			const int alloc_n_slots = _n_slots;

			transaction.unlock();
			Slot *new_slots = (Slot *) malloc(sizeof(Slot) * (alloc_n_slots + _n_grow));
			transaction.lock();

			if (new_slots == nullptr) {
				return false;
			}

			if (_n_slots != alloc_n_slots) {
				free(new_slots);
				continue;
			}

			Slot *previous_slots = _slots.load();
			memcpy(new_slots, previous_slots, sizeof(Slot) * _n_slots);

			for (int i = _n_slots; i < _n_slots + _n_grow; i++) {
				new_slots[i] = {UINT16_MAX, param_value_u{}};
			}

			_slots.store(new_slots);
			_n_slots += _n_grow;

			transaction.unlock();
			free(previous_slots);
			transaction.lock();
		}

		return _next_slot < _n_slots;
	}

	int _next_slot = 0;
	int _n_slots = 0;
	const int _n_grow;
	const int _n_prealloc;
	const bool _owned;
	px4::atomic<Slot *> _slots{nullptr};
};
