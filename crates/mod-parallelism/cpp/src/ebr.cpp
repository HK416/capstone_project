#include "ebr.h"

#include <stdexcept>


Sptr::Sptr(): sptr { 0 } {

}


void Sptr::set_ptr(LfNode* ptr) {
    sptr = reinterpret_cast<long long>(ptr);
}

LfNode* Sptr::get_ptr() {
    return reinterpret_cast<LfNode*>(sptr & 0xFFFFFFFFFFFFFFFE);
}

LfNode* Sptr::get_ptr(bool* removed) {
    long long p = sptr;
    *removed = (1 == (p & 1));
    return reinterpret_cast<LfNode*>(p & 0xFFFFFFFFFFFFFFFE);
}

bool Sptr::get_removed() {
    return (1 == (sptr & 1));
}

bool Sptr::cas(LfNode* old_p, LfNode* new_p, bool old_m, bool new_m) {
    long long old_v = reinterpret_cast<long long>(old_p);
    if(true == old_m) old_v = old_v | 1;
    else
        old_v = old_v & 0xFFFFFFFFFFFFFFFE;
    long long new_v = reinterpret_cast<long long>(new_p);
    if(true == new_m) new_v = new_v | 1;
    else
        new_v = new_v & 0xFFFFFFFFFFFFFFFE;
    return std::atomic_compare_exchange_strong(&sptr, &old_v, new_v);
}


LfNode::LfNode(const int& v):
    key { v },
    ebr_number { 0 }
{

}


Ebr::Accessor::Accessor(Ebr* ebr, int accessor_idx):
    ebr { ebr },
    accessor_idx { accessor_idx }
{

}

void Ebr::Accessor::reuse(LfNode* node) {
    ebr->reuse(accessor_idx, node);
}

void Ebr::Accessor::start_epoch() {
    ebr->start_epoch(accessor_idx);
}

void Ebr::Accessor::end_epoch() {
    ebr->end_epoch(accessor_idx);
}

LfNode* Ebr::Accessor::get_node(const int& x) {
    return ebr->get_node(accessor_idx, x);
}


Ebr::Ebr(int max_threads):
    epoch_counter { 1 },
    epoch_array(max_threads),
    free_queue(max_threads),
    accessor_counter { 0 }
{

}

Ebr::~Ebr() {
    clear();
}

void Ebr::clear() {
    for(auto& fq : free_queue) {
        while(false == fq.empty()) {
            delete fq.front();
            fq.pop();
        }
    }
    epoch_counter = 1;
}

void Ebr::reset_accessor_counter() {
    accessor_counter = 0;
}

Ebr::Accessor Ebr::get_accessor() {
    int accessor_idx = accessor_counter.load();
    accessor_counter++;

    if(accessor_idx >= epoch_array.size()) {
        throw std::out_of_range("Accessor index out of range");
    }

    return Accessor { this, accessor_idx };
}

void Ebr::reuse(int idx, LfNode* node) {
    node->ebr_number = epoch_counter;
    free_queue[idx].push(node);
}

void Ebr::start_epoch(int idx) {
    int epoch = epoch_counter++;
    epoch_array[idx].value = epoch;
}

void Ebr::end_epoch(int idx) {
    epoch_array[idx].value = 0;
}

LfNode* Ebr::get_node(int idx, const int& x) {
    if(true == free_queue[idx].empty()) {
        return new LfNode { x };
    }

    LfNode* p = free_queue[idx].front();
    for(auto& ea : epoch_array) {
        int epoch = ea.value;
        if((epoch != 0) && (epoch < p->ebr_number)) {
            return new LfNode { x };
        }
    }
    free_queue[idx].pop();
    p->key = x;
    std::atomic_thread_fence(std::memory_order_seq_cst);
    p->next.set_ptr(nullptr);
    return p;
}
