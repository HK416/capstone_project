#include "ebr.h"

extern thread_local int thread_id;


extern "C" {
    Ebr* ebr_new(int max_threads) {
        return new Ebr { max_threads };
    }
    
    // 테스트용 Set
    EbrLfSet* set_new() {
        return new EbrLfSet { };
    }
    void set_delete(EbrLfSet* set) {
        delete set;
    }
    
    void set_clear(EbrLfSet* set) {
        set->clear();
    }
    
    void set_add(EbrLfSet* set, int x) {
        set->add(x);
    }
    void set_remove(EbrLfSet* set, int x) {
        set->remove(x);
    }
    bool set_contains(EbrLfSet* set, int x) {
        return set->contains(x);
    }

    void set_thread_local_id(int id) {
        thread_id = id;
    }
}