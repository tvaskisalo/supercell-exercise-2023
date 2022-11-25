## Exercise for supercell internship for summer 2023


### How to run the solutions

Pre-requisites: To run the solutions, you should have cargo and rust installed.

Solution to the exercise 1 is in the directory solution-1 and solution to the exercise 2 is in the directory solution-2.

You can run the test by going to the corresponding directories and running.
Please note that input1.txt needs to exist in the directory for the solution-2 tests.

```
cargo test
```

To build the solutions, execute 
```
cargo build --release
```
in the corresponding directories.


To run the compiled solution-1, execute in the directory solution-1:
```
./target/release/solution-1 -i FILENAME 
```
Where: FILENAME is the name of the file.


To run the compiled solution-2, execute in the directory solution-2:
```
./target/release/solution-2 -i FILENAME THREAD_COUNT INFO WRITE
```
Where: FILENAME is the name of the file.\
(Optional) THREAD_COUNT is the number of threads the solution will use. Default value is 4.\
(Optional) INFO is 0 or 1, 1 if you want to see the execution time and 0 if not. Default value is 0.\
(Optional) WRITE is 0 or 1, 1 if you want the solution to write the output to a file. Default value is 0. This is just used for testing.

### Design choices

#### Exercise 1

Since the make_friends and del_friends are assumed to be in order without timestamp, I chose not to use threads for this exercise. Threads, by design, do not guarantee that messages would come in order. For example if thread 1 sends a make_friends message for users ab" and "bc" and thread 2 sends a del_friends message for users "ab" and "bc" at the same time. Now we cannot know which message was supposed to be sent first. So it is inconclusive wheter users "ab" and "bc" should be friends or not. Even though we can assume that they come in order, that would be a false assumption if the program, by design, does not guarantee that.


#### Exercise 2

I assumed that the input file is correct, so no input validation is done for the file. This is just for the sake of simplicity.
I made a rather controversial assumption that we know all the users in the input file, so I hardcoded the following users to the solution: 
ab, bc, cd, de, ef, fg, gh, hi, ij, jk, ba, cb, dc, ed, fe, gf, hg, ih, ji, kj. I assume that the user management is done by another services in a real case. 

What this assumption allows me to do, is to set each user to a hashmap, with the key being the user's name and the value being the user's state. For thread safety, the user's state is inside a mutex, so only one thread can access the state at a time. For example, thread 1 can access ab's state and thread 2 can access bc's state at the same time but thread 3 can't access ab's state while thread 1 is still processing. If I would make it possible to add more users to the state, I would have to put the whole data structure inside another mutex that allows concurrent reading, but locks when a thread is modifying the key/value pairs. This certainly is possible, but unfortunately I did not have the time to do it. Because the keys are never modified, the keys are thread safe by default.

This implementation is thread safe, since refrences to the hashmap's values are behind atomically counted refrences and inside a mutex. Atomically counted refrences ensure that no data is dropped while a thread is running and mutex ensures that no data is being accessed by two threads at the same time. Since a thread will process only one user's data at a time, there is no possibility for two threads to wait for each other. Only case for a deadlock is when a mutex is poisoned, but under the assumption that the input is always correct, this will not happen. Because the mutexes are dependent on the user, the threads most likely do not have to wait for other threads. Of course in the case that both threads need to process same user's data, then a thread has to wait. But when the amount of users grow, the probablitiy decreases (see benchmarks and analysis). This way we should be able to good performance gain with threads. Everywhere where data is transferred only references are actually transferred to increase performance.


### Exercise 2 benchmarks

Some benchmarks on the exercise 2:
NOTE: In Exercise 2, the program reads the file first and then processes it, which makes it slightly faster, since timing is only done only on the processing of the data.
NOTE 2: I only had a quad-core processor without hyper-threading, so I could not test the scaling with more than 4 threads. And with 4 threads the processor can't always be processing the threads, since it also has to do other things. So the scaling is most likely even better than what I got.

Sample size is 10.

Input1: 1 000 000 lines and 10 users.

1 Thread: min: 612ms, max: 630ms, average: 615ms\
2 Threads: min: 368ms, max: 375ms, average: 372ms\
3 Threads: min: 270ms, max: 273ms, average: 272ms\
4 Threads: min: 237ms, max: 271ms, average: 246ms

Input2: 2 000 000 lines and 10 users.

1 Thread: min: 1.23s, max: 1.24s, average: 1.23s \
2 Threads: min: 742ms, max: 764ms, average: 746ms\
3 Threads: min: 537ms, max: 548ms, average: 543ms\
4 Threads: min: 474ms, max: 494ms, average: 478ms

Input3: 2 000 000 lines and 20 users.

1 Thread: min: 1.23s, max: 1.32s, average: 1.27s \
2 Threads: min: 638ms, max: 643ms, average: 639ms\
3 Threads: min: 521ms, max: 535ms, average: 526ms\
4 Threads: min: 433ms, max: 439ms, average: 435ms

Input4: 4 000 000 lines and 20 users.

1 Thread: min: 2.45s, max: 2.5s, average: 2.46s\
2 Threads: min: 1.27s, max: 1.31s, average: 1.28s\
3 Threads: min: 1.05s, max: 1.18s, average: 1.07s\
4 Threads: min: 869ms, max: 897ms, average: 874ms

input2.txt, input3.txt and input4.txt are in the extra-inputs-solution-2.zip file.
Please note that input2.txt, input3.txt, and input4.txt are large files, around 100-250Mb.

#### Exercise 2 benchmark analysis

Due to the limited amount of threads, the speedup is difficult to analyse. But with this data we could approximate the speedup to be close to linear, since the execution time is near-linearly decreasing when the thread count is increasing. The time to execute with four threads is consistantly roughly third of the time compared to executing on one thread. This is probably better when running on a system with more than four processing cores. Another trend that can be seen, which I already hypotized is that the execution time gets better when the amount of users increase. When comparing the execution time between input2 and input3, we can see that with multiple threads, the execution time is lower in input3, even though the input size is the same. This is due to the decrease in probabilty that threads have to wait for each other. If we would have only one user the execution time with 4 threads would be the same with one thread. But the point of concurrent systems is that they scale great when the input size is large and in this situation a user will not make that many requests each second, instead a lot of users make small requests, so this approach scales well with that.



