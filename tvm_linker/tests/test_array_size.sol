pragma solidity ^0.5.0;

contract Test28 {

    function tvm_logstr(bytes32 logstr) private {}
    function tvm_init_storage() private {}

    uint[] arr;

    function main(uint16 starting_len, uint16 new_len) public payable returns (uint) {
        tvm_init_storage();
        for (uint i = 0; i < starting_len; i++) {
            arr[i] = i + 1;
        }
        require(arr.length == starting_len);
        arr.length = new_len;
        require(arr.length == new_len);
        if (new_len < starting_len) {
            for (uint i = new_len; i < starting_len; i++) {
                require(arr[i] == 0);
            }
        }
        return arr.length;
    }
}