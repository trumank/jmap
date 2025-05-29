class Container {
public:
    asdf asdf;
    
};

template<typename T>
struct TArray {
    public:
    T* Data;
    int Num;
    int Max;
};

template<typename T>
struct TArrayInline {
    public:
    T Inline[4];
    T* Data;
    int Num;
    int Max;
};

struct FString {
    public:
    TArray<char> String;
};

template<typename T>
struct TSet {
    public:
    TArray<T> Inner;
};

template<typename T, typename S>
class Vector : public Container {
public:
    T* data;
    int size;
    virtual void push_back(T element);
    T& at(int index);

private:
    int capacity;
    void resize();
};

struct Point {
public:
    float x;
    float y;
    Point add(Point other);
};

            class Base {
                public:
                    virtual void foo();
            };
            
            class Derived : public Base {
                public:
                    void foo();
            };
