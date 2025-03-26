#ifndef __EBR_H__
#define __EBR_H__


#include <vector>
#include <atomic>


class LfNode;

class Sptr {
private:
    std::atomic<long long> sptr;

public:
    Sptr();

public:
    void set_ptr(LfNode* ptr);
    LfNode* get_ptr();
    LfNode* get_ptr(bool* removed);
    bool get_removed();
    bool cas(LfNode* old_p, LfNode* new_p, bool old_m, bool new_m);
};

class LfNode {
public:
    int key;
    Sptr next;
    int ebr_number;

public:
    LfNode(int v);
};


struct alignas(64) AlignedAtomicInt {
    std::atomic_int value;
};

class Ebr {
private:
    std::atomic_int epoch_counter;
    alignas(64) std::vector<AlignedAtomicInt> epoch_array;

public:
    Ebr(int max_threads);
    ~Ebr();
    
public:
    void clear();
    void reuse(LfNode* node);
    void start_epoch();
    void end_epoch();

    LfNode* get_node(int x);
};


class EbrLfSet {
private:
    LfNode head;
    LfNode tail;
    alignas(64) Ebr ebr;

public:
    EbrLfSet();

public:
    void clear();
    void find(int x, LfNode*& prev, LfNode*& curr);
    bool add(int x);
    bool remove(int x);
    bool contains(int x);
};


#endif // __EBR_H__