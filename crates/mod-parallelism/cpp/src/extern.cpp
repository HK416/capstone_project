#include "ebr.h"
#include "ebr_set.h"


extern "C" {
    Ebr* ebr_new(int max_threads) {
        return new Ebr { max_threads };
    }
    
    // 테스트용 Set
    EbrLfSet* set_new(int max_threads) {
        return new EbrLfSet { max_threads };
    }
    void set_delete(EbrLfSet* set) {
        delete set;
    }
    
    void set_clear(EbrLfSet* set) {
        set->clear();
    }
    void set_reset_accessor_counter(EbrLfSet* set) {
        set->reset_accessor_counter();
    }
    /// thread-safe하지 않은 contains
    bool set_contains(EbrLfSet* set, int key) {
        return set->contains(key);
    }
    EbrLfSet::Accessor* set_new_accessor(EbrLfSet* set) {
        return set->new_accessor();
    }

    bool set_accessor_add(EbrLfSet::Accessor* accessor, int key) {
        return accessor->add(key);
    }
    bool set_accessor_remove(EbrLfSet::Accessor* accessor, int key) {
        return accessor->remove(key);
    }
    /// thread-safe한 contains
    bool set_accessor_contains(EbrLfSet::Accessor* accessor, int key) {
        return accessor->contains(key);
    }
    void set_accessor_delete(EbrLfSet::Accessor* accessor) {
        delete accessor;
    }
}