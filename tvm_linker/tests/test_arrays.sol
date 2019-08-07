pragma solidity ^0.5.0;

contract TestArray {

    function main(uint8 idx, uint32[] memory myarray) public pure returns (uint32) {
        return myarray[idx];
    }
    
}
